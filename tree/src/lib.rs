//! oracle Tree Builder — Game tree construction from configuration
//!
//! Generates a `GameTree` from a `GameConfig` specifying board, pot, stacks,
//! and bet/raise sizes. Supports flop, turn, and river starting boards.

mod actions;
mod builder;

use oracle_engine::node::{Card, GameTree};

/// Configuration for building a game tree.
#[derive(Debug, Clone)]
pub struct GameConfig {
    /// Board cards (3 = flop, 4 = turn, 5 = river)
    pub board: Vec<Card>,
    /// Initial pot size (bb)
    pub pot: f64,
    /// Remaining stacks [IP, OOP] (bb)
    pub stacks: [f64; 2],
    /// Bet sizes as pot fractions, per street [flop, turn, river]
    pub bet_sizes: [Vec<f64>; 3],
    /// Raise sizes as pot fractions, per street [flop, turn, river]
    pub raise_sizes: [Vec<f64>; 3],
    /// Maximum raises allowed per street
    pub max_raises_per_street: u32,
    /// Snap to all-in when remainder < threshold * pot
    pub allin_threshold: f64,
}

/// Errors from tree construction.
#[derive(Debug)]
pub enum TreeBuildError {
    /// Board must have 3, 4, or 5 cards
    InvalidBoardLength(usize),
    /// Board contains duplicate cards
    DuplicateBoardCards,
    /// Stacks must be > 0
    InvalidStacks,
    /// Must have at least one bet size on at least one street
    NoBetSizes,
}

impl std::fmt::Display for TreeBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TreeBuildError::InvalidBoardLength(n) => {
                write!(f, "board must have 3, 4, or 5 cards, got {}", n)
            }
            TreeBuildError::DuplicateBoardCards => write!(f, "board contains duplicate cards"),
            TreeBuildError::InvalidStacks => write!(f, "stacks must be > 0"),
            TreeBuildError::NoBetSizes => write!(f, "must have at least one bet size"),
        }
    }
}

impl GameConfig {
    /// Create a default 100bb configuration for the given board.
    ///
    /// - 75% pot bet, 100% pot raise
    /// - Max 1 raise per street
    /// - All-in threshold: 10% of pot
    pub fn default_100bb(board: Vec<Card>) -> Self {
        GameConfig {
            board,
            pot: 6.0,
            stacks: [97.0, 97.0],
            bet_sizes: [vec![0.75], vec![0.75], vec![0.75]],
            raise_sizes: [vec![1.0], vec![1.0], vec![1.0]],
            max_raises_per_street: 1,
            allin_threshold: 0.1,
        }
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), TreeBuildError> {
        let n = self.board.len();
        if n != 3 && n != 4 && n != 5 {
            return Err(TreeBuildError::InvalidBoardLength(n));
        }

        // Check for duplicate board cards
        for i in 0..self.board.len() {
            for j in (i + 1)..self.board.len() {
                if self.board[i].value() == self.board[j].value() {
                    return Err(TreeBuildError::DuplicateBoardCards);
                }
            }
        }

        if self.stacks[0] <= 0.0 || self.stacks[1] <= 0.0 {
            return Err(TreeBuildError::InvalidStacks);
        }

        Ok(())
    }
}

/// Build a game tree from the given configuration.
///
/// Returns a `GameTree` with all decision, chance, and terminal nodes.
/// The tree uses a public-tree representation: terminal nodes have `hole_cards: [None, None]`.
pub fn build_tree(config: &GameConfig) -> Result<GameTree, TreeBuildError> {
    config.validate()?;
    Ok(builder::TreeBuilder::new(config).build())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn river_board() -> Vec<Card> {
        vec![Card::new(0), Card::new(13), Card::new(26), Card::new(39), Card::new(4)]
    }

    #[test]
    fn test_validate_good_config() {
        let config = GameConfig::default_100bb(river_board());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_bad_board_length() {
        let config = GameConfig::default_100bb(vec![Card::new(0), Card::new(1)]);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_duplicate_board() {
        let config = GameConfig::default_100bb(vec![Card::new(0), Card::new(0), Card::new(1)]);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_stacks() {
        let mut config = GameConfig::default_100bb(river_board());
        config.stacks = [0.0, 97.0];
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_build_tree_river() {
        let config = GameConfig::default_100bb(river_board());
        let tree = build_tree(&config).unwrap();
        assert!(!tree.is_empty());

        let decision_count = tree.nodes.iter().filter(|n| n.is_decision()).count();
        let terminal_count = tree.nodes.iter().filter(|n| n.is_terminal()).count();
        assert!(decision_count > 0);
        assert!(terminal_count > 0);
    }

    #[test]
    fn test_build_tree_turn() {
        let board = vec![Card::new(0), Card::new(13), Card::new(26), Card::new(39)];
        let config = GameConfig::default_100bb(board);
        let tree = build_tree(&config).unwrap();

        let chance_count = tree.nodes.iter().filter(|n| n.is_chance()).count();
        assert!(chance_count > 0, "turn tree should have chance nodes");
    }
}
