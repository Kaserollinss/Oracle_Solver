//! Hardcoded 9-node test tree for Phase 2 CFR+ validation
//!
//! Represents a minimal heads-up flop spot:
//!   Board: As Kh 7d  |  Pot: 10bb  |  Stacks: [95, 95] (IP=index 0, OOP=index 1)
//!
//! Terminal EVs are fixed (from IP's perspective, in bb) — no hand evaluator needed.
//!
//! Tree structure:
//!   0: Decision OOP  [Check → 1, Bet(5) → 6]
//!   1: Decision IP   [Check → 2, Bet(5) → 3]
//!   2: Terminal      OOP chk / IP chk showdown        IP EV = +1.0
//!   3: Decision OOP  [Fold → 4, Call → 5]
//!   4: Terminal      OOP chk / IP bet / OOP fold      IP EV = +5.0
//!   5: Terminal      OOP chk / IP bet / OOP call show IP EV = +2.0
//!   6: Decision IP   [Fold → 7, Call → 8]
//!   7: Terminal      OOP bet / IP fold                IP EV = -5.0
//!   8: Terminal      OOP bet / IP call showdown        IP EV = -1.0

use std::collections::HashMap;
use crate::node::{Action, Card, GameTree, Node, NodeId, Player, Street};

/// Card encoding: suit * 13 + rank  (suit: 0=spades,1=hearts,2=diamonds,3=clubs; rank: 0=2..12=A)
fn card(suit: u8, rank: u8) -> Card {
    Card::new(suit * 13 + rank)
}

/// Build the 9-node test tree.
/// Nodes are pushed in ID order so that `tree.nodes[id] == node with id`.
pub fn build_test_tree() -> GameTree {
    let board = vec![
        card(0, 12), // As
        card(1, 11), // Kh
        card(2, 5),  // 7d
    ];
    let pot = 10.0_f64;
    let stacks = [95.0_f64, 95.0_f64]; // [IP, OOP]

    let mut nodes: Vec<Node> = Vec::with_capacity(9);

    // Node 0: Decision OOP — root
    nodes.push(Node::Decision {
        id: 0,
        infoset_id: 0,
        player: Player::OOP,
        street: Street::Flop,
        parent: None,
        children: vec![1, 6],
        actions: vec![Action::Check, Action::Bet { size: 5.0 }],
        pot,
        stacks,
        board: board.clone(),
        bet_sequence: vec![],
    });

    // Node 1: Decision IP — OOP checked
    nodes.push(Node::Decision {
        id: 1,
        infoset_id: 1,
        player: Player::IP,
        street: Street::Flop,
        parent: Some(0),
        children: vec![2, 3],
        actions: vec![Action::Check, Action::Bet { size: 5.0 }],
        pot,
        stacks,
        board: board.clone(),
        bet_sequence: vec![Action::Check],
    });

    // Node 2: Terminal — OOP chk / IP chk (showdown)
    nodes.push(Node::Terminal {
        id: 2,
        parent: Some(1),
        folder: None,
        pot,
        stacks,
        board: board.clone(),
        hole_cards: [None, None],
    });

    // Node 3: Decision OOP — OOP chk / IP bet 5
    nodes.push(Node::Decision {
        id: 3,
        infoset_id: 3,
        player: Player::OOP,
        street: Street::Flop,
        parent: Some(1),
        children: vec![4, 5],
        actions: vec![Action::Fold, Action::Call],
        pot: pot + 5.0,
        stacks: [stacks[0], stacks[1] - 5.0], // IP bet 5
        board: board.clone(),
        bet_sequence: vec![Action::Check, Action::Bet { size: 5.0 }],
    });

    // Node 4: Terminal — OOP chk / IP bet / OOP fold
    nodes.push(Node::Terminal {
        id: 4,
        parent: Some(3),
        folder: Some(Player::OOP),
        pot: pot + 5.0,
        stacks: [stacks[0], stacks[1] - 5.0],
        board: board.clone(),
        hole_cards: [None, None],
    });

    // Node 5: Terminal — OOP chk / IP bet / OOP call (showdown)
    nodes.push(Node::Terminal {
        id: 5,
        parent: Some(3),
        folder: None,
        pot: pot + 10.0,
        stacks: [stacks[0] - 5.0, stacks[1] - 5.0],
        board: board.clone(),
        hole_cards: [None, None],
    });

    // Node 6: Decision IP — OOP bet 5
    nodes.push(Node::Decision {
        id: 6,
        infoset_id: 6,
        player: Player::IP,
        street: Street::Flop,
        parent: Some(0),
        children: vec![7, 8],
        actions: vec![Action::Fold, Action::Call],
        pot: pot + 5.0,
        stacks: [stacks[0], stacks[1] - 5.0], // OOP bet 5
        board: board.clone(),
        bet_sequence: vec![Action::Bet { size: 5.0 }],
    });

    // Node 7: Terminal — OOP bet / IP fold
    nodes.push(Node::Terminal {
        id: 7,
        parent: Some(6),
        folder: Some(Player::IP),
        pot: pot + 5.0,
        stacks: [stacks[0], stacks[1] - 5.0],
        board: board.clone(),
        hole_cards: [None, None],
    });

    // Node 8: Terminal — OOP bet / IP call (showdown)
    nodes.push(Node::Terminal {
        id: 8,
        parent: Some(6),
        folder: None,
        pot: pot + 10.0,
        stacks: [stacks[0] - 5.0, stacks[1] - 5.0],
        board: board.clone(),
        hole_cards: [None, None],
    });

    GameTree { nodes }
}

/// Fixed terminal EVs from IP's perspective (in bb).
/// Keyed by NodeId of terminal nodes.
pub fn terminal_ev_table() -> HashMap<NodeId, f64> {
    let mut table = HashMap::new();
    table.insert(2, 1.0);  // OOP chk / IP chk showdown
    table.insert(4, 5.0);  // OOP chk / IP bet / OOP fold → IP wins
    table.insert(5, 2.0);  // OOP chk / IP bet / OOP call showdown
    table.insert(7, -5.0); // OOP bet / IP fold → OOP wins
    table.insert(8, -1.0); // OOP bet / IP call showdown
    table
}

/// Build the 11-node chance-tree for testing parallel CFR+ traversal.
///
/// Includes a Chance node at ID 1, branching into two independent subtrees.
/// Terminal EVs are fixed (from IP's perspective, in bb).
///
/// Tree structure:
///   0:  Decision OOP  [Check → 1, Bet(5) → 8]
///   1:  Chance        [CardA → 2, CardB → 5]
///   2:  Decision IP   [Check → 3, Bet(5) → 4]
///   3:  Terminal      EV = +1.0
///   4:  Terminal      EV = +3.0
///   5:  Decision IP   [Check → 6, Bet(5) → 7]
///   6:  Terminal      EV = +0.5
///   7:  Terminal      EV = +2.0
///   8:  Decision IP   [Fold → 9, Call → 10]
///   9:  Terminal      EV = -5.0
///   10: Terminal      EV = -1.0
pub fn build_test_tree_chance() -> GameTree {
    let board = vec![
        card(0, 12), // As
        card(1, 11), // Kh
        card(2, 5),  // 7d
    ];
    let pot = 10.0_f64;
    let stacks = [95.0_f64, 95.0_f64];

    let mut nodes: Vec<Node> = Vec::with_capacity(11);

    // Node 0: Decision OOP — root
    nodes.push(Node::Decision {
        id: 0,
        infoset_id: 0,
        player: Player::OOP,
        street: Street::Flop,
        parent: None,
        children: vec![1, 8],
        actions: vec![Action::Check, Action::Bet { size: 5.0 }],
        pot,
        stacks,
        board: board.clone(),
        bet_sequence: vec![],
    });

    // Node 1: Chance — OOP checked, abstract card dealt
    nodes.push(Node::Chance {
        id: 1,
        parent: Some(0),
        children: vec![2, 5],
        street: Street::Flop,
        pot,
        stacks,
        board: board.clone(),
    });

    // Node 2: Decision IP — after CardA
    nodes.push(Node::Decision {
        id: 2,
        infoset_id: 2,
        player: Player::IP,
        street: Street::Flop,
        parent: Some(1),
        children: vec![3, 4],
        actions: vec![Action::Check, Action::Bet { size: 5.0 }],
        pot,
        stacks,
        board: board.clone(),
        bet_sequence: vec![Action::Check],
    });

    // Node 3: Terminal — CardA / IP check EV = +1.0
    nodes.push(Node::Terminal {
        id: 3,
        parent: Some(2),
        folder: None,
        pot,
        stacks,
        board: board.clone(),
        hole_cards: [None, None],
    });

    // Node 4: Terminal — CardA / IP bet EV = +3.0
    nodes.push(Node::Terminal {
        id: 4,
        parent: Some(2),
        folder: None,
        pot: pot + 5.0,
        stacks: [stacks[0] - 5.0, stacks[1]],
        board: board.clone(),
        hole_cards: [None, None],
    });

    // Node 5: Decision IP — after CardB
    nodes.push(Node::Decision {
        id: 5,
        infoset_id: 5,
        player: Player::IP,
        street: Street::Flop,
        parent: Some(1),
        children: vec![6, 7],
        actions: vec![Action::Check, Action::Bet { size: 5.0 }],
        pot,
        stacks,
        board: board.clone(),
        bet_sequence: vec![Action::Check],
    });

    // Node 6: Terminal — CardB / IP check EV = +0.5
    nodes.push(Node::Terminal {
        id: 6,
        parent: Some(5),
        folder: None,
        pot,
        stacks,
        board: board.clone(),
        hole_cards: [None, None],
    });

    // Node 7: Terminal — CardB / IP bet EV = +2.0
    nodes.push(Node::Terminal {
        id: 7,
        parent: Some(5),
        folder: None,
        pot: pot + 5.0,
        stacks: [stacks[0] - 5.0, stacks[1]],
        board: board.clone(),
        hole_cards: [None, None],
    });

    // Node 8: Decision IP — OOP bet 5
    nodes.push(Node::Decision {
        id: 8,
        infoset_id: 8,
        player: Player::IP,
        street: Street::Flop,
        parent: Some(0),
        children: vec![9, 10],
        actions: vec![Action::Fold, Action::Call],
        pot: pot + 5.0,
        stacks: [stacks[0], stacks[1] - 5.0],
        board: board.clone(),
        bet_sequence: vec![Action::Bet { size: 5.0 }],
    });

    // Node 9: Terminal — OOP bet / IP fold EV = -5.0
    nodes.push(Node::Terminal {
        id: 9,
        parent: Some(8),
        folder: Some(Player::IP),
        pot: pot + 5.0,
        stacks: [stacks[0], stacks[1] - 5.0],
        board: board.clone(),
        hole_cards: [None, None],
    });

    // Node 10: Terminal — OOP bet / IP call showdown EV = -1.0
    nodes.push(Node::Terminal {
        id: 10,
        parent: Some(8),
        folder: None,
        pot: pot + 10.0,
        stacks: [stacks[0] - 5.0, stacks[1] - 5.0],
        board: board.clone(),
        hole_cards: [None, None],
    });

    GameTree { nodes }
}

/// Fixed terminal EVs for the chance tree, from IP's perspective (in bb).
pub fn terminal_ev_table_chance() -> HashMap<NodeId, f64> {
    let mut table = HashMap::new();
    table.insert(3, 1.0);   // CardA / IP check
    table.insert(4, 3.0);   // CardA / IP bet
    table.insert(6, 0.5);   // CardB / IP check
    table.insert(7, 2.0);   // CardB / IP bet
    table.insert(9, -5.0);  // OOP bet / IP fold
    table.insert(10, -1.0); // OOP bet / IP call
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::Node;

    #[test]
    fn test_tree_node_count() {
        let tree = build_test_tree();
        assert_eq!(tree.len(), 9);
    }

    #[test]
    fn test_root_is_oop_decision() {
        let tree = build_test_tree();
        match tree.get(0).unwrap() {
            Node::Decision { player, actions, children, .. } => {
                assert_eq!(*player, Player::OOP);
                assert_eq!(actions.len(), 2);
                assert_eq!(children, &[1u32, 6u32]);
            }
            _ => panic!("Node 0 should be a Decision node"),
        }
    }

    #[test]
    fn test_all_children_valid() {
        let tree = build_test_tree();
        for node in &tree.nodes {
            for &child_id in node.children() {
                assert!(
                    tree.get(child_id).is_some(),
                    "child id {} is out of bounds",
                    child_id
                );
            }
        }
    }

    #[test]
    fn test_decision_nodes_have_two_actions() {
        let tree = build_test_tree();
        let decision_ids = [0u32, 1, 3, 6];
        for id in decision_ids {
            match tree.get(id).unwrap() {
                Node::Decision { actions, children, .. } => {
                    assert_eq!(actions.len(), 2, "node {} should have 2 actions", id);
                    assert_eq!(children.len(), 2, "node {} should have 2 children", id);
                }
                _ => panic!("node {} should be a Decision node", id),
            }
        }
    }

    #[test]
    fn test_terminal_nodes_are_terminal() {
        let tree = build_test_tree();
        for id in [2u32, 4, 5, 7, 8] {
            assert!(
                matches!(tree.get(id).unwrap(), Node::Terminal { .. }),
                "node {} should be terminal",
                id
            );
        }
    }

    #[test]
    fn test_node_ids_match_array_index() {
        let tree = build_test_tree();
        for (idx, node) in tree.nodes.iter().enumerate() {
            assert_eq!(node.id() as usize, idx, "node id mismatch at index {}", idx);
        }
    }

    #[test]
    fn test_terminal_ev_table_coverage() {
        let table = terminal_ev_table();
        for id in [2u32, 4, 5, 7, 8] {
            assert!(table.contains_key(&id), "EV table missing entry for node {}", id);
        }
    }

    // --- Chance tree tests ---

    #[test]
    fn test_chance_tree_node_count() {
        let tree = build_test_tree_chance();
        assert_eq!(tree.len(), 11);
    }

    #[test]
    fn test_chance_tree_node_ids_match_index() {
        let tree = build_test_tree_chance();
        for (idx, node) in tree.nodes.iter().enumerate() {
            assert_eq!(node.id() as usize, idx, "chance tree node id mismatch at index {}", idx);
        }
    }

    #[test]
    fn test_chance_tree_has_chance_node() {
        let tree = build_test_tree_chance();
        assert!(
            matches!(tree.get(1).unwrap(), Node::Chance { .. }),
            "node 1 should be a Chance node"
        );
    }

    #[test]
    fn test_chance_tree_all_children_valid() {
        let tree = build_test_tree_chance();
        for node in &tree.nodes {
            for &child_id in node.children() {
                assert!(
                    tree.get(child_id).is_some(),
                    "chance tree child id {} is out of bounds",
                    child_id
                );
            }
        }
    }

    #[test]
    fn test_chance_tree_ev_table_coverage() {
        let table = terminal_ev_table_chance();
        for id in [3u32, 4, 6, 7, 9, 10] {
            assert!(table.contains_key(&id), "chance EV table missing entry for node {}", id);
        }
    }
}
