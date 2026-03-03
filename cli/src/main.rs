//! oracle CLI - Command-line interface for oracle Solver
//!
//! This binary provides a CLI harness for testing engine functionality
//! before UI integration.

use oracle_engine::evaluator::benchmark_throughput;
use oracle_engine::node::Card;
use oracle_engine::{CfrSolver, RangeSolver, compute_exploitability};
use oracle_engine::test_tree::build_test_tree;
use oracle_tree::{build_tree, GameConfig};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 3 && args[1] == "bench" && args[2] == "evaluator" {
        // Run evaluator benchmark
        println!("Running hand evaluator benchmark...");
        let sample_size = if args.len() >= 4 {
            args[3].parse().unwrap_or(1_000_000)
        } else {
            1_000_000
        };

        println!("Sample size: {} hands", sample_size);
        let (evals_per_sec, duration_ms) = benchmark_throughput(sample_size);

        println!("Results:");
        println!("  Duration: {} ms", duration_ms);
        println!("  Throughput: {:.2} evals/sec", evals_per_sec);
        println!("  Throughput: {:.2}M evals/sec", evals_per_sec / 1_000_000.0);

    } else if args.len() >= 2 && args[1] == "solve-tree" {
        run_solve_tree(&args[2..]);

    } else if args.len() >= 2 && args[1] == "solve" {
        // Parse optional flags
        let mut max_iterations: u64 = 10_000;
        let mut threshold: f64 = 0.01;
        let mut check_every: u64 = 100;
        let mut time_cap_secs: u64 = 60;

        let mut i = 2usize;
        while i < args.len() {
            match args[i].as_str() {
                "--iterations" => {
                    if i + 1 < args.len() {
                        max_iterations = args[i + 1].parse().unwrap_or(10_000);
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--threshold" => {
                    if i + 1 < args.len() {
                        threshold = args[i + 1].parse().unwrap_or(0.01);
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--check-every" => {
                    if i + 1 < args.len() {
                        check_every = args[i + 1].parse().unwrap_or(100);
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--time-cap" => {
                    if i + 1 < args.len() {
                        time_cap_secs = args[i + 1].parse().unwrap_or(60);
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                _ => {
                    i += 1;
                }
            }
        }

        run_solve(max_iterations, threshold, check_every, time_cap_secs);

    } else {
        println!("oracle Solver CLI v{}", env!("CARGO_PKG_VERSION"));
        println!("Phase 3 - Tree Builder + Range CFR");
        println!();
        println!("Usage:");
        println!("  oracle bench evaluator [sample_size]");
        println!("  oracle solve [options]");
        println!("  oracle solve-tree --board BOARD [options]");
        println!();
        println!("Commands:");
        println!("  bench evaluator          Run hand evaluator throughput benchmark");
        println!("  solve                    Solve the test tree via CFR+ and report convergence");
        println!("  solve-tree               Build and solve a real game tree with range-aware CFR");
        println!();
        println!("solve-tree options:");
        println!("  --board BOARD            Board cards, e.g. AhKs7d2c5s (required)");
        println!("  --pot P                  Initial pot in bb (default: 6.0)");
        println!("  --stacks S               Effective stacks in bb (default: 97.0)");
        println!("  --bet-sizes B            Comma-separated pot fractions (default: 0.75)");
        println!("  --raise-sizes R          Comma-separated pot fractions (default: 1.0)");
        println!("  --max-raises N           Max raises per street (default: 1)");
        println!("  --iterations N           Max CFR+ iterations (default: 1000)");
        println!("  --threshold T            Exploitability threshold in bb (default: 0.5)");
        println!("  --check-every N          Check exploitability every N iters (default: 100)");
        println!("  --time-cap S             Stop after S seconds (default: 300)");
        println!();
        println!("Examples:");
        println!("  oracle bench evaluator");
        println!("  oracle solve --iterations 5000 --threshold 0.005");
        println!("  oracle solve-tree --board AhKs7d2c5s");
        println!("  oracle solve-tree --board AhKs7d2c --iterations 500 --pot 10 --stacks 50");
    }
}

fn run_solve_tree(args: &[String]) {
    use std::time::Instant;

    // Defaults
    let mut board_str: Option<String> = None;
    let mut pot = 6.0;
    let mut stacks = 97.0;
    let mut bet_sizes_str = "0.75".to_string();
    let mut raise_sizes_str = "1.0".to_string();
    let mut max_raises: u32 = 1;
    let mut max_iterations: u64 = 1000;
    let mut threshold: f64 = 0.5;
    let mut check_every: u64 = 100;
    let mut time_cap_secs: u64 = 300;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--board" => {
                if i + 1 < args.len() {
                    board_str = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--pot" => {
                if i + 1 < args.len() {
                    pot = args[i + 1].parse().unwrap_or(6.0);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--stacks" => {
                if i + 1 < args.len() {
                    stacks = args[i + 1].parse().unwrap_or(97.0);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--bet-sizes" => {
                if i + 1 < args.len() {
                    bet_sizes_str = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--raise-sizes" => {
                if i + 1 < args.len() {
                    raise_sizes_str = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--max-raises" => {
                if i + 1 < args.len() {
                    max_raises = args[i + 1].parse().unwrap_or(1);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--iterations" => {
                if i + 1 < args.len() {
                    max_iterations = args[i + 1].parse().unwrap_or(1000);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--threshold" => {
                if i + 1 < args.len() {
                    threshold = args[i + 1].parse().unwrap_or(0.5);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--check-every" => {
                if i + 1 < args.len() {
                    check_every = args[i + 1].parse().unwrap_or(100);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--time-cap" => {
                if i + 1 < args.len() {
                    time_cap_secs = args[i + 1].parse().unwrap_or(300);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                i += 1;
            }
        }
    }

    let board_str = match board_str {
        Some(s) => s,
        None => {
            eprintln!("Error: --board is required");
            eprintln!("Usage: oracle solve-tree --board AhKs7d2c5s");
            std::process::exit(1);
        }
    };

    // Parse board
    let board = match Card::parse_board(&board_str) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error parsing board '{}': {}", board_str, e);
            std::process::exit(1);
        }
    };

    let board_len = board.len();
    if board_len != 3 && board_len != 4 && board_len != 5 {
        eprintln!(
            "Error: board must have 3 (flop), 4 (turn), or 5 (river) cards, got {}",
            board_len
        );
        std::process::exit(1);
    }

    // Parse bet/raise sizes
    let bet_sizes: Vec<f64> = bet_sizes_str
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let raise_sizes: Vec<f64> = raise_sizes_str
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    // Build config
    let config = GameConfig {
        board: board.clone(),
        pot,
        stacks: [stacks, stacks],
        bet_sizes: [bet_sizes.clone(), bet_sizes.clone(), bet_sizes.clone()],
        raise_sizes: [raise_sizes.clone(), raise_sizes.clone(), raise_sizes.clone()],
        max_raises_per_street: max_raises,
        allin_threshold: 0.1,
    };

    let street_name = match board_len {
        3 => "flop",
        4 => "turn",
        5 => "river",
        _ => "unknown",
    };

    // Print board
    let board_display: Vec<String> = board.iter().map(|c| format!("{}", c)).collect();
    println!("Board: {} ({})", board_display.join(" "), street_name);
    println!(
        "Config: pot={:.1}bb, stacks={:.1}bb, bets={:?}, raises={:?}, max_raises={}",
        pot, stacks, bet_sizes, raise_sizes, max_raises
    );
    println!();

    // Build tree
    println!("Building game tree...");
    let build_start = Instant::now();
    let tree = match build_tree(&config) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error building tree: {}", e);
            std::process::exit(1);
        }
    };
    let build_time = build_start.elapsed();

    let decision_count = tree.nodes.iter().filter(|n| n.is_decision()).count();
    let chance_count = tree.nodes.iter().filter(|n| n.is_chance()).count();
    let terminal_count = tree.nodes.iter().filter(|n| n.is_terminal()).count();

    println!(
        "Tree: {} nodes ({} decision, {} chance, {} terminal) built in {}ms",
        tree.len(),
        decision_count,
        chance_count,
        terminal_count,
        build_time.as_millis()
    );

    // Create solver
    println!("Initializing range-aware CFR solver...");
    let init_start = Instant::now();
    let mut solver = RangeSolver::new(tree, &board);
    let init_time = init_start.elapsed();

    println!(
        "Solver: {} hand combos, initialized in {}ms",
        solver.num_hands(),
        init_time.as_millis()
    );

    // Memory report
    let mem = solver.memory_usage();
    println!("Memory: {}", mem);
    println!();

    // Solve
    println!(
        "Running range-aware CFR+ (max {} iterations, threshold={:.4} bb, time cap={}s)...",
        max_iterations, threshold, time_cap_secs
    );
    println!();
    println!(
        "{:>8}  {:>16}  {:>10}  {:>10}  {:>10}",
        "Iter", "Exploitability", "IP BR", "OOP BR", "Elapsed"
    );
    println!(
        "{:->8}  {:->16}  {:->10}  {:->10}  {:->10}",
        "", "", "", "", ""
    );

    let solve_start = Instant::now();
    let time_cap = std::time::Duration::from_secs(time_cap_secs);

    let mut stop_reason = "iteration cap";
    let mut final_iter = max_iterations;
    let mut final_exploit = None;

    for iter in 1..=max_iterations {
        solver.run_iteration();

        let elapsed = solve_start.elapsed();
        let hit_time_cap = elapsed >= time_cap;
        let hit_check = iter % check_every == 0;

        if hit_check || hit_time_cap || iter == max_iterations {
            let (exploit, ip_br, oop_br) = solver.compute_exploitability();
            println!(
                "{:>8}  {:>16.6}  {:>10.6}  {:>10.6}  {:>8.1}s",
                iter,
                exploit,
                ip_br,
                oop_br,
                elapsed.as_secs_f64()
            );

            if exploit < threshold {
                stop_reason = "exploitability threshold";
                final_iter = iter;
                final_exploit = Some((exploit, ip_br, oop_br));
                break;
            }

            if hit_time_cap {
                stop_reason = "time cap";
                final_iter = iter;
                final_exploit = Some((exploit, ip_br, oop_br));
                break;
            }

            final_exploit = Some((exploit, ip_br, oop_br));
            final_iter = iter;
        }
    }

    println!();
    println!("Stopped at iteration {} ({}).", final_iter, stop_reason);

    if let Some((exploit, ip_br, oop_br)) = final_exploit {
        println!("Final exploitability : {:.6} bb", exploit);
        println!("  IP BR              : {:.6} bb", ip_br);
        println!("  OOP BR             : {:.6} bb", oop_br);
    }

    println!(
        "Total elapsed        : {:.1}s",
        solve_start.elapsed().as_secs_f64()
    );
}

fn run_solve(max_iterations: u64, threshold: f64, check_every: u64, time_cap_secs: u64) {
    use std::time::Instant;

    let tree = build_test_tree();
    let num_nodes = tree.len();
    let decision_count = tree.nodes.iter().filter(|n| n.is_decision()).count();

    println!(
        "Running CFR+ on test tree ({} nodes, {} decision nodes)...",
        num_nodes, decision_count
    );
    println!("  Max iterations : {}", max_iterations);
    println!("  Threshold      : {} bb", threshold);
    println!("  Check every    : {} iters", check_every);
    println!("  Time cap       : {} s", time_cap_secs);
    println!();
    println!(
        "{:>8}  {:>16}  {:>10}  {:>10}  {:>10}",
        "Iter", "Exploitability", "IP BR", "OOP BR", "Elapsed"
    );
    println!(
        "{:->8}  {:->16}  {:->10}  {:->10}  {:->10}",
        "", "", "", "", ""
    );

    let mut solver = CfrSolver::new(tree.clone());
    let start = Instant::now();
    let time_cap = std::time::Duration::from_secs(time_cap_secs);

    let mut stop_reason = "iteration cap";
    let mut final_iter = max_iterations;
    let mut final_metrics = None;

    for iter in 1..=max_iterations {
        solver.run_iteration();

        let elapsed = start.elapsed();

        let hit_time_cap = elapsed >= time_cap;
        let hit_check = iter % check_every == 0;

        if hit_check || hit_time_cap {
            let m = compute_exploitability(&tree, &solver.storage, iter, elapsed);
            println!(
                "{:>8}  {:>16.6}  {:>10.6}  {:>10.6}  {:>8}ms",
                iter,
                m.exploitability,
                m.ip_br_value,
                m.oop_br_value,
                elapsed.as_millis()
            );

            if m.exploitability < threshold {
                stop_reason = "exploitability threshold";
                final_iter = iter;
                final_metrics = Some(m);
                break;
            }

            if hit_time_cap {
                stop_reason = "time cap";
                final_iter = iter;
                final_metrics = Some(m);
                break;
            }

            final_metrics = Some(m);
            final_iter = iter;
        }
    }

    println!();
    println!("Stopped at iteration {} ({}).", final_iter, stop_reason);

    if let Some(m) = final_metrics {
        println!("Final exploitability : {:.6} bb", m.exploitability);
        println!("  IP BR              : {:.6} bb", m.ip_br_value);
        println!("  OOP BR             : {:.6} bb", m.oop_br_value);
        println!("Elapsed              : {} ms", m.elapsed_time.as_millis());
    }
}
