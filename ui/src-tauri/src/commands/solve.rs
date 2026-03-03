use std::sync::atomic::Ordering;
use std::time::Instant;

use oracle_engine::node::Card;
use oracle_engine::RangeSolver;
use oracle_tree::{build_tree, GameConfig};
use tauri::Emitter;

use crate::events::{SolveComplete, SolveProgress};
use crate::state::AppState;
use crate::types::*;

#[tauri::command]
#[specta::specta]
pub fn start_solve(
    config: SolveConfig,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<TreeInfoDto, String> {
    log::info!("start_solve called: {:?}", config.game.board);
    let start = Instant::now();

    // Parse board
    let board = Card::parse_board(&config.game.board)?;
    let board_len = board.len();
    if board_len != 3 && board_len != 4 && board_len != 5 {
        return Err(format!(
            "Board must have 3, 4, or 5 cards, got {}",
            board_len
        ));
    }

    // Build GameConfig
    let gc = GameConfig {
        board: board.clone(),
        pot: config.game.pot,
        stacks: [config.game.stacks, config.game.stacks],
        bet_sizes: [
            config.game.bet_sizes.clone(),
            config.game.bet_sizes.clone(),
            config.game.bet_sizes.clone(),
        ],
        raise_sizes: [
            config.game.raise_sizes.clone(),
            config.game.raise_sizes.clone(),
            config.game.raise_sizes.clone(),
        ],
        max_raises_per_street: config.game.max_raises_per_street,
        allin_threshold: config.game.allin_threshold,
    };

    // Update status
    {
        let mut session = state.lock().map_err(|e| format!("Lock error: {}", e))?;
        session.status = SolveStatus::Building;
        session.reset_cancel();
    }

    // Build tree
    log::info!("Building game tree...");
    let tree = build_tree(&gc).map_err(|e| format!("Tree build error: {:?}", e))?;

    let decision_count = tree.nodes.iter().filter(|n| n.is_decision()).count();
    let chance_count = tree.nodes.iter().filter(|n| n.is_chance()).count();
    let terminal_count = tree.nodes.iter().filter(|n| n.is_terminal()).count();
    let total_nodes = tree.len();

    log::info!(
        "Tree built: {} nodes ({} decision, {} chance, {} terminal) in {:.0}ms",
        total_nodes,
        decision_count,
        chance_count,
        terminal_count,
        start.elapsed().as_millis()
    );

    // Create solver
    log::info!("Initializing RangeSolver...");
    let solver = RangeSolver::new(tree, &board);
    let num_hands = solver.num_hands();
    let mem = solver.memory_usage();

    let memory_dto = MemoryReportDto {
        tree_mb: mem.tree_bytes as f64 / (1024.0 * 1024.0),
        storage_mb: mem.storage_bytes as f64 / (1024.0 * 1024.0),
        rank_cache_mb: mem.rank_cache_bytes as f64 / (1024.0 * 1024.0),
        conflict_table_mb: mem.conflict_table_bytes as f64 / (1024.0 * 1024.0),
        total_mb: mem.total_bytes as f64 / (1024.0 * 1024.0),
    };

    let tree_info = TreeInfoDto {
        total_nodes,
        decision_nodes: decision_count,
        chance_nodes: chance_count,
        terminal_nodes: terminal_count,
        num_hands,
        memory: memory_dto,
    };

    log::info!(
        "Solver initialized: {} hands, {:.1} MB total",
        num_hands,
        mem.total_bytes as f64 / (1024.0 * 1024.0)
    );

    // Store solver in state and spawn solve thread
    let cancel_flag = {
        let mut session = state.lock().map_err(|e| format!("Lock error: {}", e))?;
        session.solver = Some(solver);
        session.tree_info = Some(tree_info.clone());
        session.status = SolveStatus::Solving;
        session.cancel_flag.clone()
    };

    let state_clone = state.inner().clone();
    let max_iterations = config.max_iterations;
    let threshold = config.threshold;
    let check_every = config.check_every;
    let time_cap = std::time::Duration::from_secs(config.time_cap_secs);

    std::thread::spawn(move || {
        log::info!("Solve thread started: max_iter={}, threshold={:.4}", max_iterations, threshold);
        let solve_start = Instant::now();

        for iter in 1..=max_iterations {
            // Check cancellation
            log::info!("Iteration {} started", iter);
            if cancel_flag.load(Ordering::Relaxed) {
                log::info!("Solve cancelled at iteration {}", iter);
                let progress = {
                    let mut session = state_clone.lock().unwrap();
                    if let Some(ref solver) = session.solver {
                        let (exploit, ip_br, oop_br) = solver.compute_exploitability();
                        let p = SolveProgressDto {
                            iteration: iter - 1,
                            max_iterations,
                            exploitability: exploit,
                            ip_br,
                            oop_br,
                            elapsed_secs: solve_start.elapsed().as_secs_f64(),
                            status: SolveStatus::Stopped,
                        };
                        session.status = SolveStatus::Stopped;
                        session.last_progress = Some(p.clone());
                        p
                    } else {
                        return;
                    }
                };
                let _ = app.emit_to(tauri::EventTarget::any(), "solve-complete", SolveComplete(progress));
                return;
            }

            // Run one iteration
            {
                let mut session = state_clone.lock().unwrap();
                if let Some(ref mut solver) = session.solver {
                    solver.run_iteration();
                } else {
                    return;
                }
            }

            let elapsed = solve_start.elapsed();
            let hit_time_cap = elapsed >= time_cap;
            let hit_check = iter % check_every == 0;

            if hit_check || hit_time_cap || iter == max_iterations {
                let (exploit, ip_br, oop_br, converged) = {
                    let session = state_clone.lock().unwrap();
                    if let Some(ref solver) = session.solver {
                        let (e, ip, oop) = solver.compute_exploitability();
                        (e, ip, oop, e < threshold)
                    } else {
                        return;
                    }
                };

                log::debug!(
                    "iter={}, exploit={:.6}, ip_br={:.6}, oop_br={:.6}, elapsed={:.1}s",
                    iter, exploit, ip_br, oop_br, elapsed.as_secs_f64()
                );

                let status = if converged {
                    SolveStatus::Converged
                } else if hit_time_cap {
                    SolveStatus::Stopped
                } else if iter == max_iterations {
                    SolveStatus::Stopped
                } else {
                    SolveStatus::Solving
                };

                let progress = SolveProgressDto {
                    iteration: iter,
                    max_iterations,
                    exploitability: exploit,
                    ip_br,
                    oop_br,
                    elapsed_secs: elapsed.as_secs_f64(),
                    status: status.clone(),
                };

                // Update state
                {
                    let mut session = state_clone.lock().unwrap();
                    session.status = status.clone();
                    session.last_progress = Some(progress.clone());
                }

                // Emit progress event
                let _ = app.emit_to(tauri::EventTarget::any(), "solve-progress", SolveProgress(progress.clone()));

                if converged || hit_time_cap || iter == max_iterations {
                    log::info!(
                        "Solve finished: iter={}, exploit={:.6}, reason={:?}",
                        iter, exploit, status
                    );
                    let _ = app.emit_to(tauri::EventTarget::any(), "solve-complete", SolveComplete(progress));
                    return;
                }
            }
        }
    });

    Ok(tree_info)
}

#[tauri::command]
#[specta::specta]
pub fn stop_solve(state: tauri::State<'_, AppState>) -> Result<(), String> {
    log::info!("stop_solve called");
    let session = state.lock().map_err(|e| format!("Lock error: {}", e))?;
    session.cancel_flag.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_solve_status(state: tauri::State<'_, AppState>) -> Result<SolveProgressDto, String> {
    let session = state.lock().map_err(|e| format!("Lock error: {}", e))?;
    match &session.last_progress {
        Some(p) => Ok(p.clone()),
        None => Ok(SolveProgressDto {
            iteration: 0,
            max_iterations: 0,
            exploitability: 0.0,
            ip_br: 0.0,
            oop_br: 0.0,
            elapsed_secs: 0.0,
            status: session.status.clone(),
        }),
    }
}
