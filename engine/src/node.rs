//! Node definitions for the game tree
//!
//! This module defines the core Node types that represent positions in the
//! poker game tree. Nodes are designed to be immutable and separate from
//! solver state (regrets, strategies).

/// Represents a playing card (0-51, where 0-12 are spades, 13-25 are hearts, etc.)
/// or a more structured representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card(u8);

impl Card {
    /// Create a new card from a value 0-51
    pub fn new(value: u8) -> Self {
        assert!(value < 52, "Card value must be 0-51");
        Card(value)
    }

    /// Get the raw card value (0-51)
    pub fn value(self) -> u8 {
        self.0
    }
}

/// Hand rank for poker evaluation
/// 
/// Lower values represent stronger hands (e.g., Royal Flush = 1, High Card = 7462)
/// This matches standard poker hand ranking conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HandRank(u16);

impl HandRank {
    /// Create a new hand rank
    pub fn new(rank: u16) -> Self {
        HandRank(rank)
    }

    /// Get the raw rank value
    pub fn value(self) -> u16 {
        self.0
    }
}

/// Hand evaluator trait
/// 
/// This interface will be implemented in Phase 1. The evaluator is called
/// during terminal node EV calculation to determine hand strength.
pub trait HandEvaluator {
    /// Evaluate a 7-card hand (5 board cards + 2 hole cards)
    /// 
    /// Returns a HandRank where lower values represent stronger hands.
    /// The evaluator should be optimized for high throughput (target: 50M+ evals/sec).
    fn evaluate(&self, board: [Card; 5], hand: [Card; 2]) -> HandRank;
}

/// Player position in heads-up poker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Player {
    /// In Position (acts last)
    IP,
    /// Out of Position (acts first)
    OOP,
}

impl Player {
    /// Get the opponent of this player
    pub fn opponent(self) -> Player {
        match self {
            Player::IP => Player::OOP,
            Player::OOP => Player::IP,
        }
    }
}

/// Street in postflop poker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Street {
    /// Flop (3 board cards)
    Flop,
    /// Turn (4 board cards)
    Turn,
    /// River (5 board cards)
    River,
}

/// Action type available at a decision node
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// Fold (only available when facing a bet)
    Fold,
    /// Check (only available when no bet to call)
    Check,
    /// Call (only available when facing a bet)
    Call,
    /// Bet a specific size (in big blinds or pot fraction)
    Bet { size: f64 },
}

/// Node ID type (index into flat array storage)
pub type NodeId = u32;

/// Information set ID type
/// 
/// In heads-up postflop with perfect recall, each node maps 1:1 to an information set.
pub type InfosetId = u32;

/// Represents a node in the game tree
/// 
/// Nodes are immutable and contain only game state information.
/// Solver state (regrets, strategies) is stored separately in parallel arrays
/// indexed by InfosetId or NodeId.
#[derive(Debug, Clone)]
pub enum Node {
    /// Decision node where a player must act
    Decision {
        /// Unique identifier for this node (index in flat array)
        id: NodeId,
        /// Information set ID (same as id in heads-up perfect recall)
        infoset_id: InfosetId,
        /// Player to act
        player: Player,
        /// Current street
        street: Street,
        /// Parent node ID (None for root)
        parent: Option<NodeId>,
        /// Child node IDs indexed by action
        children: Vec<NodeId>,
        /// Available actions at this node
        actions: Vec<Action>,
        /// Current pot size (in big blinds)
        pot: f64,
        /// Stack sizes for each player (in big blinds)
        stacks: [f64; 2],
        /// Board cards (0-5 cards depending on street)
        board: Vec<Card>,
        /// Bet sequence leading to this node (for reconstruction if needed)
        bet_sequence: Vec<Action>,
    },
    /// Chance node where board cards are dealt
    Chance {
        /// Unique identifier for this node
        id: NodeId,
        /// Parent node ID
        parent: Option<NodeId>,
        /// Child node IDs (one per possible board card)
        children: Vec<NodeId>,
        /// Current street before chance event
        street: Street,
        /// Pot size
        pot: f64,
        /// Stack sizes
        stacks: [f64; 2],
        /// Board cards before this chance event
        board: Vec<Card>,
    },
    /// Terminal node (showdown or fold)
    Terminal {
        /// Unique identifier for this node
        id: NodeId,
        /// Parent node ID
        parent: Option<NodeId>,
        /// Player who folded (None if showdown)
        folder: Option<Player>,
        /// Final pot size
        pot: f64,
        /// Final stack sizes
        stacks: [f64; 2],
        /// Final board cards (0-5 cards)
        board: Vec<Card>,
        /// Hole cards for each player (needed for EV calculation)
        /// Index 0 = IP, Index 1 = OOP
        hole_cards: [Option<[Card; 2]>; 2],
    },
}

impl Node {
    /// Get the node ID
    pub fn id(&self) -> NodeId {
        match self {
            Node::Decision { id, .. } => *id,
            Node::Chance { id, .. } => *id,
            Node::Terminal { id, .. } => *id,
        }
    }

    /// Get the information set ID (only valid for Decision nodes)
    pub fn infoset_id(&self) -> Option<InfosetId> {
        match self {
            Node::Decision { infoset_id, .. } => Some(*infoset_id),
            _ => None,
        }
    }

    /// Get the parent node ID
    pub fn parent(&self) -> Option<NodeId> {
        match self {
            Node::Decision { parent, .. } => *parent,
            Node::Chance { parent, .. } => *parent,
            Node::Terminal { parent, .. } => *parent,
        }
    }

    /// Get child node IDs
    pub fn children(&self) -> &[NodeId] {
        match self {
            Node::Decision { children, .. } => children,
            Node::Chance { children, .. } => children,
            Node::Terminal { .. } => &[],
        }
    }

    /// Get the current street
    pub fn street(&self) -> Option<Street> {
        match self {
            Node::Decision { street, .. } => Some(*street),
            Node::Chance { street, .. } => Some(*street),
            Node::Terminal { .. } => None,
        }
    }

    /// Get board cards (for evaluator interface)
    pub fn board(&self) -> &[Card] {
        match self {
            Node::Decision { board, .. } => board,
            Node::Chance { board, .. } => board,
            Node::Terminal { board, .. } => board,
        }
    }

    /// Check if this is a terminal node
    pub fn is_terminal(&self) -> bool {
        matches!(self, Node::Terminal { .. })
    }

    /// Check if this is a decision node
    pub fn is_decision(&self) -> bool {
        matches!(self, Node::Decision { .. })
    }

    /// Check if this is a chance node
    pub fn is_chance(&self) -> bool {
        matches!(self, Node::Chance { .. })
    }
}

/// Game tree wrapper
/// 
/// Contains a flat array of nodes for efficient traversal and cache locality.
#[derive(Debug, Clone)]
pub struct GameTree {
    /// Flat array of nodes indexed by NodeId
    pub nodes: Vec<Node>,
}

impl GameTree {
    /// Create a new empty game tree
    pub fn new() -> Self {
        GameTree { nodes: Vec::new() }
    }

    /// Get a node by ID
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id as usize)
    }

    /// Get a mutable reference to a node by ID
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id as usize)
    }

    /// Get the number of nodes
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the tree is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for GameTree {
    fn default() -> Self {
        Self::new()
    }
}
