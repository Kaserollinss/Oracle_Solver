//! oracle CLI - Command-line interface for oracle Solver
//!
//! This binary provides a CLI harness for testing engine functionality
//! before UI integration.

use oracle_engine::evaluator::benchmark_throughput;
use oracle_engine::{CfrSolver, compute_exploitability};
use oracle_engine::test_tree::build_test_tree;

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
        println!("Phase 2 - CFR+ Solver");
        println!();
        println!("Usage:");
        println!("  oracle bench evaluator [sample_size]");
        println!("  oracle solve [options]");
        println!();
        println!("Commands:");
        println!("  bench evaluator          Run hand evaluator throughput benchmark");
        println!("  solve                    Solve the test tree via CFR+ and report convergence");
        println!();
        println!("Solve options:");
        println!("  --iterations N           Max CFR+ iterations (default: 10000)");
        println!("  --threshold T            Stop when exploitability < T bb (default: 0.01)");
        println!("  --check-every N          Check exploitability every N iterations (default: 100)");
        println!("  --time-cap S             Stop after S seconds (default: 60)");
        println!();
        println!("Examples:");
        println!("  oracle bench evaluator              # 1M hand benchmark");
        println!("  oracle bench evaluator 10000000     # 10M hand benchmark");
        println!("  oracle solve                        # solve with defaults");
        println!("  oracle solve --iterations 5000 --threshold 0.005");
    }
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
