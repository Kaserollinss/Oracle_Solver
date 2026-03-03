use oracle_engine::node::{Card, Node};
use oracle_engine::range_cfr::RangeSolver;

fn river_board() -> Vec<Card> {
    // As Kh 7d 2c 5s
    vec![
        Card::new(12),
        Card::new(24),
        Card::new(31),
        Card::new(39),
        Card::new(3),
    ]
}

#[test]
fn test_built_tree_structure() {
    let board = river_board();
    let config = oracle_tree::GameConfig::default_100bb(board.clone());
    let tree = oracle_tree::build_tree(&config).unwrap();

    assert_eq!(tree.nodes.len(), 27, "river tree should have 27 nodes");

    // Check pot values are sensible
    let max_pot = 6.0 + 97.0 + 97.0; // both all-in
    for node in &tree.nodes {
        if let Node::Terminal { pot, .. } = node {
            assert!(
                *pot <= max_pot + 0.01,
                "terminal pot {} exceeds max {}",
                pot,
                max_pot
            );
            assert!(*pot >= 6.0, "terminal pot {} below initial pot", pot);
        }
    }
}

#[test]
fn test_built_tree_convergence() {
    let board = river_board();
    let config = oracle_tree::GameConfig::default_100bb(board.clone());
    let tree = oracle_tree::build_tree(&config).unwrap();

    let mut solver = RangeSolver::new(tree, &board);
    let (e0, _, _) = solver.compute_exploitability();

    for _ in 0..500 {
        solver.run_iteration();
    }
    let (mid, _, _) = solver.compute_exploitability();

    for _ in 0..500 {
        solver.run_iteration();
    }
    let (final_exploit, _, _) = solver.compute_exploitability();

    // Exploitability should decrease monotonically (roughly)
    assert!(
        mid < e0 * 0.1,
        "exploit should drop 10x by iter 500: initial={e0:.4} mid={mid:.4}"
    );
    assert!(
        final_exploit < 0.1,
        "exploit should be < 0.1 bb after 1000 iters: {final_exploit:.4}"
    );
}

#[test]
fn test_range_solver_on_built_tree() {
    let board = river_board();
    let config = oracle_tree::GameConfig::default_100bb(board.clone());
    let tree = oracle_tree::build_tree(&config).unwrap();

    let mut solver = RangeSolver::new(tree, &board);

    for _ in 0..200 {
        solver.run_iteration();
    }

    let (exploit, _, _) = solver.compute_exploitability();
    assert!(
        exploit < 1.0,
        "exploit should be < 1 bb after 200 iters: {exploit:.4}"
    );
}
