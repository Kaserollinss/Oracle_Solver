use serde::{Deserialize, Serialize};
use specta::Type;

use oracle_engine::node::{Action, Card, Node, Player, Street};

// ── Card DTO ──

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct CardDto {
    pub value: u8,
    pub rank: String,
    pub suit: String,
    pub display: String,
}

impl CardDto {
    pub fn from_card(c: Card) -> Self {
        let rank = match c.rank() {
            0 => "2", 1 => "3", 2 => "4", 3 => "5", 4 => "6",
            5 => "7", 6 => "8", 7 => "9", 8 => "T", 9 => "J",
            10 => "Q", 11 => "K", 12 => "A", _ => "?",
        };
        let suit = match c.suit() {
            0 => "s", 1 => "h", 2 => "d", 3 => "c", _ => "?",
        };
        CardDto {
            value: c.value(),
            rank: rank.to_string(),
            suit: suit.to_string(),
            display: format!("{}", c),
        }
    }
}

// ── Game Config DTO ──

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct GameConfigDto {
    pub board: String,
    pub pot: f64,
    pub stacks: f64,
    pub bet_sizes: Vec<f64>,
    pub raise_sizes: Vec<f64>,
    pub max_raises_per_street: u32,
    pub allin_threshold: f64,
}

// ── Solve Config ──

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct SolveConfig {
    pub game: GameConfigDto,
    pub max_iterations: u64,
    pub threshold: f64,
    pub check_every: u64,
    pub time_cap_secs: u64,
}

// ── Solve Progress ──

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct SolveProgressDto {
    pub iteration: u64,
    pub max_iterations: u64,
    pub exploitability: f64,
    pub ip_br: f64,
    pub oop_br: f64,
    pub elapsed_secs: f64,
    pub status: SolveStatus,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
pub enum SolveStatus {
    Idle,
    Building,
    Solving,
    Converged,
    Stopped,
    Error { message: String },
}

// ── Memory Report ──

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct MemoryReportDto {
    pub tree_mb: f64,
    pub storage_mb: f64,
    pub rank_cache_mb: f64,
    pub conflict_table_mb: f64,
    pub total_mb: f64,
}

// ── Tree Info ──

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct TreeInfoDto {
    pub total_nodes: usize,
    pub decision_nodes: usize,
    pub chance_nodes: usize,
    pub terminal_nodes: usize,
    pub num_hands: usize,
    pub memory: MemoryReportDto,
}

// ── Tree Node DTO ──

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct TreeNodeDto {
    pub id: u32,
    pub node_type: String,
    pub player: Option<String>,
    pub street: Option<String>,
    pub actions: Vec<ActionDto>,
    pub children: Vec<u32>,
    pub pot: f64,
}

impl TreeNodeDto {
    pub fn from_node(node: &Node) -> Self {
        match node {
            Node::Decision {
                id, player, street, children, actions, pot, ..
            } => TreeNodeDto {
                id: *id,
                node_type: "decision".to_string(),
                player: Some(match player {
                    Player::IP => "IP".to_string(),
                    Player::OOP => "OOP".to_string(),
                }),
                street: Some(street_str(*street)),
                actions: actions.iter().map(ActionDto::from_action).collect(),
                children: children.clone(),
                pot: *pot,
            },
            Node::Chance {
                id, children, street, pot, ..
            } => TreeNodeDto {
                id: *id,
                node_type: "chance".to_string(),
                player: None,
                street: Some(street_str(*street)),
                actions: vec![],
                children: children.clone(),
                pot: *pot,
            },
            Node::Terminal { id, pot, folder, .. } => TreeNodeDto {
                id: *id,
                node_type: if folder.is_some() { "fold".to_string() } else { "showdown".to_string() },
                player: folder.map(|f| match f {
                    Player::IP => "IP".to_string(),
                    Player::OOP => "OOP".to_string(),
                }),
                street: None,
                actions: vec![],
                children: vec![],
                pot: *pot,
            },
        }
    }
}

// ── Action DTO ──

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct ActionDto {
    pub label: String,
    pub action_type: String,
    pub size: Option<f64>,
}

impl ActionDto {
    pub fn from_action(action: &Action) -> Self {
        match action {
            Action::Fold => ActionDto {
                label: "Fold".to_string(),
                action_type: "fold".to_string(),
                size: None,
            },
            Action::Check => ActionDto {
                label: "Check".to_string(),
                action_type: "check".to_string(),
                size: None,
            },
            Action::Call => ActionDto {
                label: "Call".to_string(),
                action_type: "call".to_string(),
                size: None,
            },
            Action::Bet { size } => ActionDto {
                label: format!("Bet {:.1}", size),
                action_type: "bet".to_string(),
                size: Some(*size),
            },
        }
    }
}

// ── Strategy Matrix ──

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct StrategyMatrixDto {
    pub node_id: u32,
    pub player: String,
    pub actions: Vec<ActionDto>,
    pub cells: Vec<MatrixCellDto>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct MatrixCellDto {
    pub hand_label: String,
    pub row: u8,
    pub col: u8,
    pub is_pair: bool,
    pub is_suited: bool,
    pub frequencies: Vec<f64>,
    pub ev: Option<f64>,
    pub combos: u32,
}

// ── Hand Detail ──

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct HandDetailDto {
    pub hand: String,
    pub cards: [CardDto; 2],
    pub frequencies: Vec<f64>,
    pub ev: f64,
    pub equity: Option<f64>,
}

// ── Helpers ──

fn street_str(s: Street) -> String {
    match s {
        Street::Flop => "Flop".to_string(),
        Street::Turn => "Turn".to_string(),
        Street::River => "River".to_string(),
    }
}
