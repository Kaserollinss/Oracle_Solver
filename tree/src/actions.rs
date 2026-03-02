//! Betting state and legal action generation
//!
//! Determines which actions are available at each decision point in the game tree.
//! Handles bet sizing, all-in snapping, deduplication, and min-raise enforcement.

use oracle_engine::node::{Action, Card, Player, Street};

use crate::GameConfig;

/// Tracks the current betting state at a decision point.
#[derive(Debug, Clone)]
pub(crate) struct BettingState {
    /// Player to act
    pub player: Player,
    /// Current street
    pub street: Street,
    /// Current pot size (bb)
    pub pot: f64,
    /// Remaining stacks [IP, OOP] (bb)
    pub stacks: [f64; 2],
    /// Amount the acting player must call (0.0 if no bet facing)
    pub amount_to_call: f64,
    /// Number of raises on this street so far
    pub num_raises_this_street: u32,
    /// Current board cards
    pub board: Vec<Card>,
    /// Bet sequence leading to this state
    pub bet_sequence: Vec<Action>,
    /// Last bet/raise increment (for min-raise calculation)
    pub last_raise_size: f64,
}

/// Returns the street index for indexing into bet_sizes/raise_sizes arrays.
fn street_index(street: Street) -> usize {
    match street {
        Street::Flop => 0,
        Street::Turn => 1,
        Street::River => 2,
    }
}

/// Player index: IP=0, OOP=1
fn player_index(player: Player) -> usize {
    match player {
        Player::IP => 0,
        Player::OOP => 1,
    }
}

/// Generate all legal actions for the current betting state.
///
/// Rules:
/// - No bet facing: Check + Bet(sizes) + All-in
/// - Facing bet: Fold + Call + Raise(sizes if under cap) + All-in
/// - All-in snapping: if a bet leaves < threshold * pot in stack, snap to all-in
/// - Deduplication: merge actions with identical chip amounts
/// - Min-raise: raise must add at least the last bet increment
pub(crate) fn legal_actions(state: &BettingState, config: &GameConfig) -> Vec<Action> {
    let pidx = player_index(state.player);
    let my_stack = state.stacks[pidx];
    let si = street_index(state.street);

    // Player is all-in — no actions
    if my_stack <= 1e-9 {
        return vec![];
    }

    if state.amount_to_call < 1e-9 {
        // No bet facing: Check + Bet sizes + possible All-in
        no_bet_actions(state, config, my_stack, si)
    } else {
        // Facing a bet: Fold + Call + Raise sizes + possible All-in
        facing_bet_actions(state, config, my_stack, si)
    }
}

/// Actions when no bet is facing (check/bet).
fn no_bet_actions(
    state: &BettingState,
    config: &GameConfig,
    my_stack: f64,
    si: usize,
) -> Vec<Action> {
    let mut actions: Vec<Action> = vec![Action::Check];
    let mut seen_sizes: Vec<f64> = Vec::new();

    let allin_amount = my_stack;
    let threshold = config.allin_threshold * state.pot;

    for &frac in &config.bet_sizes[si] {
        let raw_size = frac * state.pot;
        let size = if raw_size >= my_stack || (my_stack - raw_size) < threshold {
            // Snap to all-in
            allin_amount
        } else {
            raw_size
        };

        if !is_duplicate(size, &seen_sizes) {
            seen_sizes.push(size);
            actions.push(Action::Bet { size });
        }
    }

    // Explicit all-in if not already covered
    if !is_duplicate(allin_amount, &seen_sizes) {
        actions.push(Action::Bet { size: allin_amount });
    }

    actions
}

/// Actions when facing a bet (fold/call/raise).
fn facing_bet_actions(
    state: &BettingState,
    config: &GameConfig,
    my_stack: f64,
    si: usize,
) -> Vec<Action> {
    let mut actions: Vec<Action> = vec![Action::Fold];

    // Call — may be all-in call
    let call_amount = state.amount_to_call.min(my_stack);
    actions.push(Action::Call);

    // Can only raise if under the raise cap and have chips beyond calling
    let remaining_after_call = my_stack - call_amount;
    if remaining_after_call > 1e-9 && state.num_raises_this_street < config.max_raises_per_street {
        let mut seen_sizes: Vec<f64> = Vec::new();
        let allin_total = my_stack; // total chips going in (call + raise on top)
        let threshold = config.allin_threshold * (state.pot + call_amount * 2.0);

        // Min-raise: must add at least the last bet increment
        let min_raise_increment = state.last_raise_size.max(1.0);
        let min_raise_total = call_amount + min_raise_increment;

        // Pot after call (before raise) for sizing calculation
        let pot_after_call = state.pot + call_amount * 2.0;

        for &frac in &config.raise_sizes[si] {
            let raise_amount = frac * pot_after_call; // raise on top of call
            let total = call_amount + raise_amount;

            // Enforce min-raise
            let total = total.max(min_raise_total);

            let total = if total >= my_stack || (my_stack - total) < threshold {
                allin_total
            } else {
                total
            };

            if !is_duplicate(total, &seen_sizes) {
                seen_sizes.push(total);
                actions.push(Action::Bet { size: total });
            }
        }

        // Explicit all-in if not already covered
        if !is_duplicate(allin_total, &seen_sizes) {
            actions.push(Action::Bet { size: allin_total });
        }
    }

    actions
}

/// Check if a size is already in the seen list (within tolerance).
fn is_duplicate(size: f64, seen: &[f64]) -> bool {
    seen.iter().any(|&s| (s - size).abs() < 0.01)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oracle_engine::node::Card;

    fn default_config() -> GameConfig {
        GameConfig {
            board: vec![Card::new(0), Card::new(13), Card::new(26), Card::new(39), Card::new(4)],
            pot: 6.0,
            stacks: [97.0, 97.0],
            bet_sizes: [vec![0.75], vec![0.75], vec![0.75]],
            raise_sizes: [vec![1.0], vec![1.0], vec![1.0]],
            max_raises_per_street: 1,
            allin_threshold: 0.1,
        }
    }

    fn make_state(player: Player, amount_to_call: f64, pot: f64, stacks: [f64; 2]) -> BettingState {
        BettingState {
            player,
            street: Street::River,
            pot,
            stacks,
            amount_to_call,
            num_raises_this_street: 0,
            board: vec![Card::new(0), Card::new(13), Card::new(26), Card::new(39), Card::new(4)],
            bet_sequence: vec![],
            last_raise_size: 0.0,
        }
    }

    #[test]
    fn test_no_bet_actions_check_and_bet() {
        let config = default_config();
        let state = make_state(Player::OOP, 0.0, 6.0, [97.0, 97.0]);
        let actions = legal_actions(&state, &config);

        // Check + 75% pot bet + all-in
        assert!(actions.contains(&Action::Check));
        assert!(actions.len() >= 2); // Check + at least one bet
        // First non-check action should be 75% pot = 4.5
        if let Action::Bet { size } = actions[1] {
            assert!((size - 4.5).abs() < 0.01);
        } else {
            panic!("expected bet");
        }
    }

    #[test]
    fn test_facing_bet_actions() {
        let config = default_config();
        let state = BettingState {
            player: Player::IP,
            street: Street::River,
            pot: 10.5, // pot after OOP's 4.5 bet
            stacks: [97.0, 92.5],
            amount_to_call: 4.5,
            num_raises_this_street: 0,
            board: vec![Card::new(0), Card::new(13), Card::new(26), Card::new(39), Card::new(4)],
            bet_sequence: vec![],
            last_raise_size: 4.5,
        };
        let actions = legal_actions(&state, &config);

        assert!(actions.contains(&Action::Fold));
        assert!(actions.contains(&Action::Call));
        // Should have raise options
        assert!(actions.len() >= 3);
    }

    #[test]
    fn test_allin_snapping() {
        let config = default_config();
        // Small stack — any bet should snap to all-in
        let state = make_state(Player::OOP, 0.0, 6.0, [97.0, 5.0]);
        let actions = legal_actions(&state, &config);

        // Check + all-in (75% pot = 4.5, stack = 5.0, remainder = 0.5 < 0.1 * 6 = 0.6 → snap)
        assert!(actions.contains(&Action::Check));
        // All bet actions should be 5.0 (all-in)
        for a in &actions {
            if let Action::Bet { size } = a {
                assert!((*size - 5.0).abs() < 0.01, "expected all-in 5.0, got {}", size);
            }
        }
    }

    #[test]
    fn test_raise_cap() {
        let config = default_config(); // max_raises = 1
        let state = BettingState {
            player: Player::IP,
            street: Street::River,
            pot: 30.0,
            stacks: [80.0, 80.0],
            amount_to_call: 10.0,
            num_raises_this_street: 1, // already at cap
            board: vec![Card::new(0), Card::new(13), Card::new(26), Card::new(39), Card::new(4)],
            bet_sequence: vec![],
            last_raise_size: 10.0,
        };
        let actions = legal_actions(&state, &config);

        // Only Fold + Call — no raises
        assert_eq!(actions.len(), 2);
        assert!(actions.contains(&Action::Fold));
        assert!(actions.contains(&Action::Call));
    }

    #[test]
    fn test_dedup_allin() {
        // When bet size exactly equals all-in, should not duplicate
        let config = GameConfig {
            board: vec![Card::new(0), Card::new(13), Card::new(26), Card::new(39), Card::new(4)],
            pot: 10.0,
            stacks: [7.5, 7.5],
            bet_sizes: [vec![0.75], vec![0.75], vec![0.75]],
            raise_sizes: [vec![1.0], vec![1.0], vec![1.0]],
            max_raises_per_street: 1,
            allin_threshold: 0.1,
        };
        let state = make_state(Player::OOP, 0.0, 10.0, [7.5, 7.5]);
        let actions = legal_actions(&state, &config);

        // Count bet actions — should be exactly 1 (all-in)
        let bet_count = actions.iter().filter(|a| matches!(a, Action::Bet { .. })).count();
        assert_eq!(bet_count, 1, "should dedup to single all-in bet");
    }

    #[test]
    fn test_allin_stack_no_actions() {
        let config = default_config();
        let state = make_state(Player::OOP, 0.0, 6.0, [97.0, 0.0]);
        let actions = legal_actions(&state, &config);
        assert!(actions.is_empty(), "all-in player has no actions");
    }
}
