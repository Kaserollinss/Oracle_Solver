//! Recursive DFS game tree construction
//!
//! Builds a `GameTree` from a `GameConfig` by recursively generating decision,
//! chance, and terminal nodes. OOP acts first on each street.

use oracle_engine::node::{Action, Card, GameTree, Node, NodeId, Player, Street};

use crate::actions::{legal_actions, BettingState};
use crate::GameConfig;

/// DFS tree builder. Allocates nodes into a flat Vec and backpatches children.
pub(crate) struct TreeBuilder<'a> {
    config: &'a GameConfig,
    nodes: Vec<Node>,
}

impl<'a> TreeBuilder<'a> {
    pub fn new(config: &'a GameConfig) -> Self {
        TreeBuilder {
            config,
            nodes: Vec::new(),
        }
    }

    /// Build the complete game tree. Returns a `GameTree`.
    pub fn build(mut self) -> GameTree {
        let street = match self.config.board.len() {
            3 => Street::Flop,
            4 => Street::Turn,
            5 => Street::River,
            _ => panic!("board must have 3, 4, or 5 cards"),
        };

        let root_state = BettingState {
            player: Player::OOP, // OOP acts first
            street,
            pot: self.config.pot,
            stacks: self.config.stacks,
            amount_to_call: 0.0,
            num_raises_this_street: 0,
            board: self.config.board.clone(),
            bet_sequence: Vec::new(),
            last_raise_size: 0.0,
        };

        self.build_decision(root_state, None);

        GameTree { nodes: self.nodes }
    }

    /// Allocate a placeholder node, returning its ID.
    fn alloc_node(&mut self) -> NodeId {
        let id = self.nodes.len() as NodeId;
        // Push a dummy terminal — will be replaced
        self.nodes.push(Node::Terminal {
            id,
            parent: None,
            folder: None,
            pot: 0.0,
            stacks: [0.0, 0.0],
            board: Vec::new(),
            hole_cards: [None, None],
        });
        id
    }

    /// Build a decision node. Returns its NodeId.
    fn build_decision(&mut self, state: BettingState, parent: Option<NodeId>) -> NodeId {
        let actions = legal_actions(&state, self.config);

        // If no actions available (player is all-in), this shouldn't be called.
        // But handle gracefully: end of betting round.
        if actions.is_empty() {
            return self.end_of_betting_round(&state, parent);
        }

        let node_id = self.alloc_node();

        let mut children = Vec::with_capacity(actions.len());
        for &action in &actions {
            let child_id = self.apply_action(&state, node_id, action);
            children.push(child_id);
        }

        self.nodes[node_id as usize] = Node::Decision {
            id: node_id,
            infoset_id: node_id,
            player: state.player,
            street: state.street,
            parent,
            children,
            actions,
            pot: state.pot,
            stacks: state.stacks,
            board: state.board.clone(),
            bet_sequence: state.bet_sequence.clone(),
        };

        node_id
    }

    /// Apply an action and return the child NodeId.
    fn apply_action(&mut self, state: &BettingState, parent: NodeId, action: Action) -> NodeId {
        match action {
            Action::Fold => self.apply_fold(state, parent),
            Action::Check => self.apply_check(state, parent),
            Action::Call => self.apply_call(state, parent),
            Action::Bet { size } => self.apply_bet(state, parent, size),
        }
    }

    /// Fold → terminal node. The acting player folds.
    fn apply_fold(&mut self, state: &BettingState, parent: NodeId) -> NodeId {
        let node_id = self.alloc_node();
        self.nodes[node_id as usize] = Node::Terminal {
            id: node_id,
            parent: Some(parent),
            folder: Some(state.player),
            pot: state.pot,
            stacks: state.stacks,
            board: state.board.clone(),
            hole_cards: [None, None],
        };
        node_id
    }

    /// Check. If OOP checks → IP acts. If IP checks → end of betting round (check-check).
    fn apply_check(&mut self, state: &BettingState, parent: NodeId) -> NodeId {
        let mut new_state = state.clone();
        new_state.bet_sequence.push(Action::Check);

        match state.player {
            Player::OOP => {
                // OOP checked, IP acts
                new_state.player = Player::IP;
                new_state.amount_to_call = 0.0;
                self.build_decision(new_state, Some(parent))
            }
            Player::IP => {
                // Check-check → end of betting round
                self.end_of_betting_round(&new_state, Some(parent))
            }
        }
    }

    /// Call. Ends the betting round (bet-call closes the street).
    fn apply_call(&mut self, state: &BettingState, parent: NodeId) -> NodeId {
        let pidx = player_index(state.player);
        let call_amount = state.amount_to_call.min(state.stacks[pidx]);

        let mut new_state = state.clone();
        new_state.stacks[pidx] -= call_amount;
        new_state.pot += call_amount;
        new_state.bet_sequence.push(Action::Call);
        new_state.amount_to_call = 0.0;

        self.end_of_betting_round(&new_state, Some(parent))
    }

    /// Bet/raise. Update pot/stacks, opponent faces the bet.
    fn apply_bet(&mut self, state: &BettingState, parent: NodeId, size: f64) -> NodeId {
        let pidx = player_index(state.player);
        let actual_size = size.min(state.stacks[pidx]);

        let mut new_state = state.clone();
        new_state.stacks[pidx] -= actual_size;
        new_state.pot += actual_size;
        new_state.bet_sequence.push(Action::Bet { size: actual_size });

        // The opponent now faces a bet
        new_state.player = state.player.opponent();
        // The amount to call is the bet minus what was already called
        // If there was a previous bet to call, the raise is on top
        if state.amount_to_call > 0.0 {
            // This is a raise. The raise increment is actual_size - amount_to_call
            let raise_increment = actual_size - state.amount_to_call;
            new_state.amount_to_call = raise_increment;
            new_state.last_raise_size = raise_increment;
            new_state.num_raises_this_street = state.num_raises_this_street + 1;
        } else {
            // This is an opening bet
            new_state.amount_to_call = actual_size;
            new_state.last_raise_size = actual_size;
            new_state.num_raises_this_street = state.num_raises_this_street;
        }

        self.build_decision(new_state, Some(parent))
    }

    /// End of betting round. Dispatches based on street and all-in status.
    fn end_of_betting_round(&mut self, state: &BettingState, parent: Option<NodeId>) -> NodeId {
        let both_allin = state.stacks[0] < 1e-9 || state.stacks[1] < 1e-9;

        match state.street {
            Street::River => {
                // River ends → showdown
                self.build_terminal_showdown(state, parent)
            }
            Street::Flop | Street::Turn => {
                if both_allin {
                    // All-in: deal remaining streets as chance-only, then showdown
                    self.build_runout_chance(state, parent)
                } else {
                    // Next street: chance node
                    let next_street = next_street(state.street);
                    self.build_chance_node(state, parent, next_street)
                }
            }
        }
    }

    /// Build a chance node dealing one card for the next street.
    fn build_chance_node(
        &mut self,
        state: &BettingState,
        parent: Option<NodeId>,
        next_street: Street,
    ) -> NodeId {
        let node_id = self.alloc_node();
        let board_set: Vec<u8> = state.board.iter().map(|c| c.value()).collect();

        let mut children = Vec::new();
        for c in 0u8..52 {
            if board_set.contains(&c) {
                continue;
            }

            let mut new_board = state.board.clone();
            new_board.push(Card::new(c));

            let child_state = BettingState {
                player: Player::OOP,
                street: next_street,
                pot: state.pot,
                stacks: state.stacks,
                amount_to_call: 0.0,
                num_raises_this_street: 0,
                board: new_board,
                bet_sequence: Vec::new(),
                last_raise_size: 0.0,
            };

            let child_id = self.build_decision(child_state, Some(node_id));
            children.push(child_id);
        }

        self.nodes[node_id as usize] = Node::Chance {
            id: node_id,
            parent,
            children,
            street: state.street,
            pot: state.pot,
            stacks: state.stacks,
            board: state.board.clone(),
        };

        node_id
    }

    /// Build runout chance nodes for all-in situations before the river.
    /// Deals remaining streets as chance-only (no decisions), then showdown.
    fn build_runout_chance(
        &mut self,
        state: &BettingState,
        parent: Option<NodeId>,
    ) -> NodeId {
        let streets_remaining = match state.street {
            Street::Flop => 2, // need turn + river
            Street::Turn => 1, // need river
            Street::River => 0,
        };

        if streets_remaining == 0 {
            return self.build_terminal_showdown(state, parent);
        }

        let node_id = self.alloc_node();
        let board_set: Vec<u8> = state.board.iter().map(|c| c.value()).collect();

        let mut children = Vec::new();
        for c in 0u8..52 {
            if board_set.contains(&c) {
                continue;
            }

            let mut new_board = state.board.clone();
            new_board.push(Card::new(c));

            let next_st = next_street(state.street);
            let child_state = BettingState {
                player: Player::OOP,
                street: next_st,
                pot: state.pot,
                stacks: state.stacks,
                amount_to_call: 0.0,
                num_raises_this_street: 0,
                board: new_board,
                bet_sequence: Vec::new(),
                last_raise_size: 0.0,
            };

            // Recurse: if still not river, will create another chance node
            let child_id = self.build_runout_chance(&child_state, Some(node_id));
            children.push(child_id);
        }

        self.nodes[node_id as usize] = Node::Chance {
            id: node_id,
            parent,
            children,
            street: state.street,
            pot: state.pot,
            stacks: state.stacks,
            board: state.board.clone(),
        };

        node_id
    }

    /// Build a showdown terminal node.
    fn build_terminal_showdown(
        &mut self,
        state: &BettingState,
        parent: Option<NodeId>,
    ) -> NodeId {
        let node_id = self.alloc_node();
        self.nodes[node_id as usize] = Node::Terminal {
            id: node_id,
            parent,
            folder: None,
            pot: state.pot,
            stacks: state.stacks,
            board: state.board.clone(),
            hole_cards: [None, None], // public tree — no specific hands
        };
        node_id
    }
}

fn next_street(street: Street) -> Street {
    match street {
        Street::Flop => Street::Turn,
        Street::Turn => Street::River,
        Street::River => panic!("no street after river"),
    }
}

fn player_index(player: Player) -> usize {
    match player {
        Player::IP => 0,
        Player::OOP => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GameConfig;

    fn river_board() -> Vec<Card> {
        vec![Card::new(0), Card::new(13), Card::new(26), Card::new(39), Card::new(4)]
    }

    fn river_config() -> GameConfig {
        GameConfig {
            board: river_board(),
            pot: 6.0,
            stacks: [97.0, 97.0],
            bet_sizes: [vec![0.75], vec![0.75], vec![0.75]],
            raise_sizes: [vec![1.0], vec![1.0], vec![1.0]],
            max_raises_per_street: 1,
            allin_threshold: 0.1,
        }
    }

    #[test]
    fn test_river_tree_node_ids_match_indices() {
        let config = river_config();
        let tree = TreeBuilder::new(&config).build();
        for (i, node) in tree.nodes.iter().enumerate() {
            assert_eq!(node.id() as usize, i, "node id mismatch at index {}", i);
        }
    }

    #[test]
    fn test_river_tree_has_terminals() {
        let config = river_config();
        let tree = TreeBuilder::new(&config).build();
        let terminals: Vec<_> = tree.nodes.iter().filter(|n| n.is_terminal()).collect();
        assert!(!terminals.is_empty(), "tree should have terminal nodes");
    }

    #[test]
    fn test_river_tree_children_valid() {
        let config = river_config();
        let tree = TreeBuilder::new(&config).build();
        let num_nodes = tree.nodes.len() as u32;
        for node in &tree.nodes {
            for &child_id in node.children() {
                assert!(child_id < num_nodes, "child {} out of range", child_id);
            }
        }
    }

    #[test]
    fn test_check_check_leads_to_showdown() {
        // With only check actions: OOP check → IP check → showdown
        let config = GameConfig {
            board: river_board(),
            pot: 6.0,
            stacks: [97.0, 97.0],
            bet_sizes: [vec![], vec![], vec![]], // no bet sizes → check-only? No, still has all-in
            raise_sizes: [vec![], vec![], vec![]],
            max_raises_per_street: 0,
            allin_threshold: 0.1,
        };
        let tree = TreeBuilder::new(&config).build();

        // Root should be decision for OOP
        if let Node::Decision { player, children, actions, .. } = &tree.nodes[0] {
            assert_eq!(*player, Player::OOP);
            // Should have Check + All-in bet
            assert!(actions.contains(&Action::Check));

            // Find the check child
            let check_idx = actions.iter().position(|a| *a == Action::Check).unwrap();
            let check_child = children[check_idx];

            // Check child should be IP's decision
            if let Node::Decision { player: ip_player, actions: ip_actions, children: ip_children, .. } = &tree.nodes[check_child as usize] {
                assert_eq!(*ip_player, Player::IP);
                assert!(ip_actions.contains(&Action::Check));

                // IP check → showdown terminal
                let ip_check_idx = ip_actions.iter().position(|a| *a == Action::Check).unwrap();
                let showdown = ip_children[ip_check_idx];
                assert!(tree.nodes[showdown as usize].is_terminal());
                if let Node::Terminal { folder, .. } = &tree.nodes[showdown as usize] {
                    assert!(folder.is_none(), "check-check should be showdown, not fold");
                }
            } else {
                panic!("expected decision node for IP after OOP check");
            }
        } else {
            panic!("root should be decision node");
        }
    }

    #[test]
    fn test_fold_terminal_has_correct_folder() {
        let config = river_config();
        let tree = TreeBuilder::new(&config).build();

        // Find fold terminals
        let fold_terminals: Vec<_> = tree.nodes.iter().filter(|n| {
            if let Node::Terminal { folder: Some(_), .. } = n { true } else { false }
        }).collect();

        assert!(!fold_terminals.is_empty(), "should have fold terminals");
    }

    #[test]
    fn test_turn_tree_has_chance_nodes() {
        let config = GameConfig {
            board: vec![Card::new(0), Card::new(13), Card::new(26), Card::new(39)], // 4-card turn board
            pot: 6.0,
            stacks: [97.0, 97.0],
            bet_sizes: [vec![0.75], vec![0.75], vec![0.75]],
            raise_sizes: [vec![1.0], vec![1.0], vec![1.0]],
            max_raises_per_street: 1,
            allin_threshold: 0.1,
        };
        let tree = TreeBuilder::new(&config).build();

        let chance_count = tree.nodes.iter().filter(|n| n.is_chance()).count();
        assert!(chance_count > 0, "turn tree should have chance nodes");

        // Each chance node should have 48 children (52 - 4 board cards)
        for node in &tree.nodes {
            if let Node::Chance { children, .. } = node {
                assert_eq!(children.len(), 48, "chance node should have 48 children");
            }
        }
    }

    #[test]
    fn test_river_tree_no_chance_nodes() {
        let config = river_config();
        let tree = TreeBuilder::new(&config).build();
        let chance_count = tree.nodes.iter().filter(|n| n.is_chance()).count();
        assert_eq!(chance_count, 0, "river tree should have no chance nodes");
    }

    #[test]
    fn test_flop_tree_has_chance_nodes() {
        // Flop board: 3 cards — should produce chance nodes for turn
        let config = GameConfig {
            board: vec![Card::new(0), Card::new(13), Card::new(26)],
            pot: 6.0,
            stacks: [97.0, 97.0],
            bet_sizes: [vec![0.75], vec![0.75], vec![0.75]],
            raise_sizes: [vec![1.0], vec![1.0], vec![1.0]],
            max_raises_per_street: 1,
            allin_threshold: 0.1,
        };
        let tree = TreeBuilder::new(&config).build();

        let chance_count = tree.nodes.iter().filter(|n| n.is_chance()).count();
        assert!(chance_count > 0, "flop tree should have chance nodes");
    }
}
