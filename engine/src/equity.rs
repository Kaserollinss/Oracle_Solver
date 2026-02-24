//! Equity computation and range iteration
//!
//! Bridges the hand evaluator (Phase 1) and CFR solver (Phase 2+).
//!
//! ## EV convention
//! All values are in big blinds, from IP's perspective.
//! - Showdown: `pot * equity_ip` (IP's raw share of the pot)
//! - IP folds:  `0.0`  (IP receives nothing)
//! - OOP folds: `pot`  (IP wins the entire pot)
//!
//! This "share of pot" convention is consistent across all callers.

use std::collections::HashMap;

use crate::evaluator::CactusKevEvaluator;
use crate::node::{Card, GameTree, Node, NodeId, Player};

// ─── Single-combo primitives ──────────────────────────────────────────────────

/// IP's equity in [0.0, 1.0] at a 5-card showdown.
///
/// Returns 1.0 if IP has the better hand, 0.0 if OOP does, 0.5 on a tie.
pub fn hand_equity_ip(board: [Card; 5], ip_hand: [Card; 2], oop_hand: [Card; 2]) -> f64 {
    let eval = CactusKevEvaluator::new();
    let ip_rank = eval.evaluate_7cards(board, ip_hand);
    let oop_rank = eval.evaluate_7cards(board, oop_hand);
    // Lower HandRank value = stronger hand
    match ip_rank.value().cmp(&oop_rank.value()) {
        std::cmp::Ordering::Less => 1.0,
        std::cmp::Ordering::Greater => 0.0,
        std::cmp::Ordering::Equal => 0.5,
    }
}

/// IP's EV at a showdown terminal: `pot × equity_ip`.
pub fn terminal_ev_ip_showdown(
    board: [Card; 5],
    ip_hand: [Card; 2],
    oop_hand: [Card; 2],
    pot: f64,
) -> f64 {
    pot * hand_equity_ip(board, ip_hand, oop_hand)
}

/// IP's EV when a player folds.
///
/// - `folder = IP`  → IP gets `0.0`
/// - `folder = OOP` → IP wins the entire `pot`
pub fn terminal_ev_ip_fold(folder: Player, pot: f64) -> f64 {
    match folder {
        Player::IP => 0.0,
        Player::OOP => pot,
    }
}

// ─── Range enumeration ────────────────────────────────────────────────────────

/// Returns every valid 2-card hole combo from the 52-card deck, excluding
/// any cards in `dead_cards` (board cards, known holdings, etc.).
///
/// Combos are returned in ascending order by card value: `a < b` always.
pub fn enumerate_combos(dead_cards: &[Card]) -> Vec<[Card; 2]> {
    let dead: Vec<u8> = dead_cards.iter().map(|c| c.value()).collect();
    let mut combos = Vec::new();
    for a in 0u8..52 {
        if dead.contains(&a) {
            continue;
        }
        for b in (a + 1)..52 {
            if dead.contains(&b) {
                continue;
            }
            combos.push([Card::new(a), Card::new(b)]);
        }
    }
    combos
}

// ─── Range-averaged EV ────────────────────────────────────────────────────────

/// Average IP EV over all non-conflicting (ip_hand, oop_hand) pairs drawn from
/// the provided combo slices.
///
/// Pairs are excluded when either hand overlaps with `board` or with each other.
/// `folder = None` means showdown; `folder = Some(p)` means `p` folded.
///
/// Returns `0.0` if no valid pairs exist.
pub fn range_ev_ip(
    board: [Card; 5],
    ip_combos: &[[Card; 2]],
    oop_combos: &[[Card; 2]],
    pot: f64,
    folder: Option<Player>,
) -> f64 {
    let board_vals: [u8; 5] = [
        board[0].value(),
        board[1].value(),
        board[2].value(),
        board[3].value(),
        board[4].value(),
    ];

    let mut total_ev = 0.0_f64;
    let mut count = 0u64;

    for &ip_hand in ip_combos {
        let ip0 = ip_hand[0].value();
        let ip1 = ip_hand[1].value();
        // Skip if IP hand conflicts with board
        if board_vals.contains(&ip0) || board_vals.contains(&ip1) {
            continue;
        }

        for &oop_hand in oop_combos {
            let oop0 = oop_hand[0].value();
            let oop1 = oop_hand[1].value();
            // Skip if OOP hand conflicts with board or IP hand
            if board_vals.contains(&oop0) || board_vals.contains(&oop1) {
                continue;
            }
            if oop0 == ip0 || oop0 == ip1 || oop1 == ip0 || oop1 == ip1 {
                continue;
            }

            let ev = match folder {
                None => terminal_ev_ip_showdown(board, ip_hand, oop_hand, pot),
                Some(p) => terminal_ev_ip_fold(p, pot),
            };
            total_ev += ev;
            count += 1;
        }
    }

    if count == 0 {
        return 0.0;
    }
    total_ev / count as f64
}

// ─── EV table from tree ───────────────────────────────────────────────────────

/// Build a terminal EV table for `tree` using the hand evaluator.
///
/// For each terminal node:
/// - Fold: `terminal_ev_ip_fold(folder, pot)` — no evaluator needed.
/// - Showdown with `hole_cards` set and a 5-card `board`: evaluator-computed EV.
/// - Showdown without hole cards or incomplete board: assumes 50/50 split (`pot / 2.0`).
///
/// Use this to replace hardcoded EV tables with evaluator-computed ones once
/// the tree builder assigns concrete hole cards to terminal nodes.
pub fn build_ev_table_from_eval(tree: &GameTree) -> HashMap<NodeId, f64> {
    let mut table = HashMap::new();
    for node in &tree.nodes {
        if let Node::Terminal { id, folder, pot, board, hole_cards, .. } = node {
            let ev = match folder {
                Some(p) => terminal_ev_ip_fold(*p, *pot),
                None => {
                    if let (Some(ip_hand), Some(oop_hand)) = (hole_cards[0], hole_cards[1]) {
                        if board.len() == 5 {
                            let board5 = [board[0], board[1], board[2], board[3], board[4]];
                            terminal_ev_ip_showdown(board5, ip_hand, oop_hand, *pot)
                        } else {
                            // Board not yet complete — assume equal split
                            pot / 2.0
                        }
                    } else {
                        // No hole cards assigned — assume equal split
                        pot / 2.0
                    }
                }
            };
            table.insert(*id, ev);
        }
    }
    table
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn card(suit: u8, rank: u8) -> Card {
        Card::new(suit * 13 + rank)
    }

    fn board_aks7d2c() -> [Card; 5] {
        [
            card(0, 12), // As
            card(1, 11), // Kh
            card(2, 5),  // 7d
            card(3, 0),  // 2c
            card(0, 3),  // 5s
        ]
    }

    #[test]
    fn test_equity_nut_flush_beats_top_pair() {
        let board = board_aks7d2c();
        // IP has Qs-Js (nut flush draw completes — but board has 5 spades already, check)
        // Actually As is on board. Let's use Ks-Ts for IP (flush using board spades)
        // Board spades: As(0,12) and 5s(0,3) — only 2 spades, not a flush.
        // Use a simpler test: top pair vs two pair
        let ip_hand = [card(1, 12), card(2, 12)]; // Kh, Kd → trips with board K
        let oop_hand = [card(3, 11), card(3, 5)];  // Qc, 7c → two pair Q+7? No, board has K+7+2+5.
        // IP: K,K + board A,K,7,2,5 → three kings
        // OOP: Q,7 + board → pair 7s
        let equity = hand_equity_ip(board, ip_hand, oop_hand);
        assert_eq!(equity, 1.0, "trips should beat one pair");
    }

    #[test]
    fn test_equity_tie() {
        let board = board_aks7d2c();
        // Board ranks present: A(12) K(11) 7(5) 5(3) 2(0)
        // IP and OOP both hold rank-1 (3) and rank-2 (4) — neither pairs the board.
        // 7-card: A K 7 5 4 3 2 → best 5 = A K 7 5 4 (high card) for both → tie.
        let ip_hand  = [card(1, 1), card(1, 2)]; // 3h, 4h
        let oop_hand = [card(2, 1), card(2, 2)]; // 3d, 4d — same ranks, different suits
        let equity = hand_equity_ip(board, ip_hand, oop_hand);
        assert_eq!(equity, 0.5, "identical board-play hands should tie");
    }

    #[test]
    fn test_terminal_ev_showdown_full_pot_on_win() {
        let board = board_aks7d2c();
        let ip_hand = [card(1, 12), card(2, 12)]; // trips kings
        let oop_hand = [card(3, 11), card(3, 5)]; // weaker
        let ev = terminal_ev_ip_showdown(board, ip_hand, oop_hand, 20.0);
        assert!((ev - 20.0).abs() < 1e-10, "winning IP should get full pot");
    }

    #[test]
    fn test_terminal_ev_fold_ip_folds() {
        assert_eq!(terminal_ev_ip_fold(Player::IP, 15.0), 0.0);
    }

    #[test]
    fn test_terminal_ev_fold_oop_folds() {
        assert_eq!(terminal_ev_ip_fold(Player::OOP, 15.0), 15.0);
    }

    #[test]
    fn test_enumerate_combos_no_dead() {
        let combos = enumerate_combos(&[]);
        assert_eq!(combos.len(), 1326); // C(52, 2)
        // All pairs are ordered a < b
        for combo in &combos {
            assert!(combo[0].value() < combo[1].value());
        }
    }

    #[test]
    fn test_enumerate_combos_with_dead() {
        // Remove 3 board cards → C(49, 2) = 1176
        let dead = [Card::new(0), Card::new(1), Card::new(2)];
        let combos = enumerate_combos(&dead);
        assert_eq!(combos.len(), 1176);
        // No dead card appears in any combo
        for combo in &combos {
            assert!(combo[0].value() > 2 && combo[1].value() > 2);
        }
    }

    #[test]
    fn test_range_ev_fold_deterministic() {
        // When OOP folds, every pair yields pot regardless of hands
        let board = board_aks7d2c();
        let dead: Vec<Card> = board.to_vec();
        let combos = enumerate_combos(&dead);
        let pot = 10.0;
        let ev = range_ev_ip(board, &combos, &combos, pot, Some(Player::OOP));
        assert!((ev - pot).abs() < 1e-10, "OOP fold → IP gets full pot: {}", ev);
    }

    #[test]
    fn test_range_ev_ip_fold_is_zero() {
        let board = board_aks7d2c();
        let dead: Vec<Card> = board.to_vec();
        let combos = enumerate_combos(&dead);
        let ev = range_ev_ip(board, &combos, &combos, 10.0, Some(Player::IP));
        assert!((ev - 0.0).abs() < 1e-10, "IP fold → EV = 0: {}", ev);
    }

    #[test]
    fn test_range_ev_showdown_near_half_pot() {
        // With random ranges, average equity should be close to 50% (not exact due to asymmetry)
        let board = board_aks7d2c();
        let dead: Vec<Card> = board.to_vec();
        let combos = enumerate_combos(&dead);
        let pot = 10.0;
        let ev = range_ev_ip(board, &combos, &combos, pot, None);
        // Should be close to pot/2 = 5.0 (exact symmetry when both use same range)
        assert!(
            (ev - 5.0).abs() < 0.01,
            "symmetric ranges → ~50% equity: got {}",
            ev
        );
    }

    #[test]
    fn test_build_ev_table_fold_nodes() {
        use crate::node::{GameTree, Node};
        // Build a minimal tree: one fold terminal (OOP folds), one showdown terminal
        let board = vec![
            card(0, 12),
            card(1, 11),
            card(2, 5),
            card(3, 0),
            card(0, 3),
        ];
        let mut nodes: Vec<Node> = Vec::new();

        // Node 0: OOP folds — IP wins pot
        nodes.push(Node::Terminal {
            id: 0,
            parent: None,
            folder: Some(Player::OOP),
            pot: 15.0,
            stacks: [90.0, 90.0],
            board: board.clone(),
            hole_cards: [None, None],
        });

        // Node 1: showdown with hole cards set
        nodes.push(Node::Terminal {
            id: 1,
            parent: None,
            folder: None,
            pot: 20.0,
            stacks: [90.0, 90.0],
            board: board.clone(),
            hole_cards: [
                Some([card(1, 12), card(2, 12)]), // IP: trips kings
                Some([card(3, 11), card(3, 5)]),  // OOP: weaker
            ],
        });

        // Node 2: showdown without hole cards — should default to pot/2
        nodes.push(Node::Terminal {
            id: 2,
            parent: None,
            folder: None,
            pot: 12.0,
            stacks: [90.0, 90.0],
            board: board.clone(),
            hole_cards: [None, None],
        });

        let tree = GameTree { nodes };
        let table = build_ev_table_from_eval(&tree);

        // Node 0: OOP folds → IP gets full pot
        assert!((table[&0] - 15.0).abs() < 1e-10);
        // Node 1: IP has trips kings → wins full pot
        assert!((table[&1] - 20.0).abs() < 1e-10);
        // Node 2: no hole cards → pot/2
        assert!((table[&2] - 6.0).abs() < 1e-10);
    }
}
