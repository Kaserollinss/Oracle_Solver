//! oracle CLI - Command-line interface for oracle Solver
//!
//! This binary provides a CLI harness for testing engine functionality
//! before UI integration.

use oracle_engine::evaluator::benchmark_throughput;

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
    } else {
        println!("oracle Solver CLI v{}", env!("CARGO_PKG_VERSION"));
        println!("Phase 1 - Hand Evaluator");
        println!();
        println!("Usage:");
        println!("  oracle bench evaluator [sample_size]");
        println!();
        println!("Examples:");
        println!("  oracle bench evaluator          # Run with 1M hands");
        println!("  oracle bench evaluator 10000000 # Run with 10M hands");
    }
}
