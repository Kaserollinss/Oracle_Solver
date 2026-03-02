//! Criterion benchmarks for CFR+ solver throughput

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use oracle_engine::cfr::CfrSolver;
use oracle_engine::exploitability::compute_exploitability;
use oracle_engine::node::Card;
use oracle_engine::range_cfr::RangeSolver;
use oracle_engine::test_tree::build_test_tree;
use oracle_tree::{build_tree, GameConfig};
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

// ─── Range CFR Benchmarks ────────────────────────────────────────────────────

fn river_board() -> Vec<Card> {
    vec![
        Card::new(12),
        Card::new(24),
        Card::new(31),
        Card::new(39),
        Card::new(3),
    ]
}

fn river_tree_and_board() -> (oracle_engine::node::GameTree, Vec<Card>) {
    let board = river_board();
    let config = GameConfig::default_100bb(board.clone());
    let tree = build_tree(&config).unwrap();
    (tree, board)
}

fn benchmark_tree_build_river(c: &mut Criterion) {
    let board = river_board();
    let config = GameConfig::default_100bb(board);
    c.bench_function("tree_build_river", |b| {
        b.iter(|| {
            black_box(build_tree(&config).unwrap());
        })
    });
}

fn benchmark_range_cfr_single_iteration(c: &mut Criterion) {
    let (tree, board) = river_tree_and_board();
    c.bench_function("range_cfr_single_iteration_river", |b| {
        b.iter_batched(
            || RangeSolver::new(tree.clone(), &board),
            |mut solver| {
                solver.run_iteration();
                black_box(&solver.storage);
            },
            BatchSize::LargeInput,
        )
    });
}

fn benchmark_range_cfr_10_iterations(c: &mut Criterion) {
    let (tree, board) = river_tree_and_board();
    c.bench_function("range_cfr_10_iterations_river", |b| {
        b.iter_batched(
            || RangeSolver::new(tree.clone(), &board),
            |mut solver| {
                for _ in 0..10 {
                    solver.run_iteration();
                }
                black_box(&solver.storage);
            },
            BatchSize::LargeInput,
        )
    });
}

fn benchmark_range_exploitability(c: &mut Criterion) {
    let (tree, board) = river_tree_and_board();
    let mut solver = RangeSolver::new(tree, &board);
    for _ in 0..10 {
        solver.run_iteration();
    }
    c.bench_function("range_exploitability_river", |b| {
        b.iter(|| black_box(solver.compute_exploitability()))
    });
}

criterion_group!(
    benches,
    benchmark_cfr_single_iteration,
    benchmark_cfr_1000_iterations,
    benchmark_exploitability_check,
    benchmark_tree_build_river,
    benchmark_range_cfr_single_iteration,
    benchmark_range_cfr_10_iterations,
    benchmark_range_exploitability,
);
criterion_main!(benches);
