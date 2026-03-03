use oracle_engine::node::Card;

use crate::state::AppState;
use crate::types::*;

const RANKS: [&str; 13] = ["A", "K", "Q", "J", "T", "9", "8", "7", "6", "5", "4", "3", "2"];

/// Map a combo [Card; 2] to (row, col) in the 13x13 grid.
///
/// Grid convention (matching standard poker range matrix):
///   - Diagonal: pocket pairs (AA at [0,0], 22 at [12,12])
///   - Above diagonal: suited hands (AKs at [0,1])
///   - Below diagonal: offsuit hands (AKo at [1,0])
fn combo_to_cell(hand: [Card; 2]) -> (u8, u8) {
    let r0 = hand[0].rank();
    let r1 = hand[1].rank();
    let (high, low) = if r0 >= r1 { (r0, r1) } else { (r1, r0) };
    let suited = hand[0].suit() == hand[1].suit();

    // rank 12=A, 11=K, ..., 0=2
    // row/col 0=A, 1=K, ..., 12=2
    if high == low {
        // Pair: on diagonal
        (12 - high, 12 - high)
    } else if suited {
        // Suited: above diagonal (row < col)
        (12 - high, 12 - low)
    } else {
        // Offsuit: below diagonal (row > col)
        (12 - low, 12 - high)
    }
}

fn cell_label(row: u8, col: u8) -> String {
    if row == col {
        format!("{}{}", RANKS[row as usize], RANKS[col as usize])
    } else if row < col {
        // suited (above diagonal)
        format!("{}{}s", RANKS[row as usize], RANKS[col as usize])
    } else {
        // offsuit (below diagonal)
        format!("{}{}o", RANKS[col as usize], RANKS[row as usize])
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_strategy_matrix(
    node_id: u32,
    state: tauri::State<'_, AppState>,
) -> Result<StrategyMatrixDto, String> {
    log::debug!("get_strategy_matrix: node_id={}", node_id);
    let session = state.lock().map_err(|e| format!("Lock error: {}", e))?;
    let solver = session
        .solver
        .as_ref()
        .ok_or_else(|| "No active solve session".to_string())?;

    let node = solver
        .tree
        .get(node_id)
        .ok_or_else(|| format!("Node {} not found", node_id))?;

    let (player, actions) = match node {
        oracle_engine::node::Node::Decision {
            player, actions, ..
        } => (player, actions),
        _ => return Err("Node is not a decision node".to_string()),
    };

    let player_str = match player {
        oracle_engine::node::Player::IP => "IP",
        oracle_engine::node::Player::OOP => "OOP",
    };

    let num_actions = actions.len();
    let action_dtos: Vec<ActionDto> = actions.iter().map(ActionDto::from_action).collect();

    // Accumulate frequencies per cell (169 cells, each with num_actions frequencies)
    let mut cell_freq_sums: Vec<Vec<f64>> = vec![vec![0.0; num_actions]; 169];
    let mut cell_counts: Vec<u32> = vec![0; 169];

    let num_hands = solver.num_hands();
    for hand_idx in 0..num_hands {
        let hand = solver.hand_at(hand_idx);
        let (row, col) = combo_to_cell(hand);
        let cell_idx = row as usize * 13 + col as usize;

        let strategy = solver.average_strategy(node_id, hand_idx);
        if strategy.len() == num_actions {
            for (a, &freq) in strategy.iter().enumerate() {
                cell_freq_sums[cell_idx][a] += freq;
            }
            cell_counts[cell_idx] += 1;
        }
    }

    // Build cells
    let mut cells = Vec::with_capacity(169);
    for row in 0..13u8 {
        for col in 0..13u8 {
            let idx = row as usize * 13 + col as usize;
            let count = cell_counts[idx];
            let frequencies = if count > 0 {
                cell_freq_sums[idx]
                    .iter()
                    .map(|&s| s / count as f64)
                    .collect()
            } else {
                vec![0.0; num_actions]
            };

            cells.push(MatrixCellDto {
                hand_label: cell_label(row, col),
                row,
                col,
                is_pair: row == col,
                is_suited: row < col,
                frequencies,
                ev: None,
                combos: count,
            });
        }
    }

    Ok(StrategyMatrixDto {
        node_id,
        player: player_str.to_string(),
        actions: action_dtos,
        cells,
    })
}

#[tauri::command]
#[specta::specta]
pub fn get_hand_detail(
    node_id: u32,
    hand_idx: usize,
    state: tauri::State<'_, AppState>,
) -> Result<HandDetailDto, String> {
    log::debug!("get_hand_detail: node_id={}, hand_idx={}", node_id, hand_idx);
    let session = state.lock().map_err(|e| format!("Lock error: {}", e))?;
    let solver = session
        .solver
        .as_ref()
        .ok_or_else(|| "No active solve session".to_string())?;

    if hand_idx >= solver.num_hands() {
        return Err(format!(
            "hand_idx {} out of range (max {})",
            hand_idx,
            solver.num_hands()
        ));
    }

    let hand = solver.hand_at(hand_idx);
    let strategy = solver.average_strategy(node_id, hand_idx);

    Ok(HandDetailDto {
        hand: format!("{}{}", hand[0], hand[1]),
        cards: [CardDto::from_card(hand[0]), CardDto::from_card(hand[1])],
        frequencies: strategy,
        ev: 0.0, // TODO: compute per-hand EV
        equity: None,
    })
}

#[tauri::command]
#[specta::specta]
pub fn get_node_ev(
    node_id: u32,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<f64>, String> {
    log::debug!("get_node_ev: node_id={}", node_id);
    let session = state.lock().map_err(|e| format!("Lock error: {}", e))?;
    let solver = session
        .solver
        .as_ref()
        .ok_or_else(|| "No active solve session".to_string())?;

    // Return placeholder EVs for now — full EV computation requires
    // an additional traversal pass (Phase 5)
    let num_hands = solver.num_hands();
    Ok(vec![0.0; num_hands])
}
