//! Criterion benchmarks for CFR+ solver throughput

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use oracle_engine::cfr::CfrSolver;
use oracle_engine::exploitability::compute_exploitability;
use oracle_engine::test_tree::build_test_tree;
use std::time::Duration;

fn benchmark_cfr_single_iteration(c: &mut Criterion) {
    c.bench_function("cfr_single_iteration", |b| {
        b.iter_batched(
            || {
                let tree = build_test_tree();
                CfrSolver::new(tree)
            },
            |mut solver| {
                solver.run_iteration();
                black_box(&solver.storage);
            },
            BatchSize::SmallInput,
        )
    });
}

fn benchmark_cfr_1000_iterations(c: &mut Criterion) {
    c.bench_function("cfr_1000_iterations", |b| {
        b.iter_batched(
            || {
                let tree = build_test_tree();
                CfrSolver::new(tree)
            },
            |mut solver| {
                for _ in 0..1_000 {
                    solver.run_iteration();
                }
                black_box(&solver.storage);
            },
            BatchSize::SmallInput,
        )
    });
}

fn benchmark_exploitability_check(c: &mut Criterion) {
    let tree = build_test_tree();
    let mut solver = CfrSolver::new(tree.clone());
    for _ in 0..1_000 {
        solver.run_iteration();
    }
    c.bench_function("exploitability_check", |b| {
        b.iter(|| {
            compute_exploitability(
                black_box(&solver.tree),
                black_box(&solver.storage),
                1_000,
                Duration::ZERO,
            )
        })
    });
}

criterion_group!(
    benches,
    benchmark_cfr_single_iteration,
    benchmark_cfr_1000_iterations,
    benchmark_exploitability_check,
);
criterion_main!(benches);
