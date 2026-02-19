//! Benchmark harness for memory layout validation
//!
//! This benchmark validates the benchmark infrastructure by iterating over
//! a small Vec<Node> structure. Real benchmarks will be added in Phase 1/2.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oracle_engine::node::{Card, GameTree, Node, NodeId, Player, Street, Action};

fn create_test_tree(size: usize) -> GameTree {
    let mut tree = GameTree::new();
    
    // Create a simple linear tree for testing
    for i in 0..size {
        let node = Node::Decision {
            id: i as NodeId,
            infoset_id: i as NodeId,
            player: if i % 2 == 0 { Player::IP } else { Player::OOP },
            street: Street::Flop,
            parent: if i > 0 { Some((i - 1) as NodeId) } else { None },
            children: if i < size - 1 { vec![(i + 1) as NodeId] } else { vec![] },
            actions: vec![Action::Check],
            pot: 100.0,
            stacks: [100.0, 100.0],
            board: vec![Card::new(0), Card::new(1), Card::new(2)],
            bet_sequence: vec![],
        };
        tree.nodes.push(node);
    }
    
    tree
}

fn benchmark_node_iteration(c: &mut Criterion) {
    let tree = create_test_tree(10_000);
    
    c.bench_function("solver_memory_layout_iteration", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for node in black_box(&tree.nodes) {
                sum += node.id() as u64;
            }
            black_box(sum)
        })
    });
}

criterion_group!(benches, benchmark_node_iteration);
criterion_main!(benches);
