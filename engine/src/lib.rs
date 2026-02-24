//! oracle Engine - Core solver types and logic
//!
//! This crate contains the core solver engine types, including Node definitions,
//! game tree structures, and (in later phases) CFR+ algorithm implementation.
//!
//! The engine is platform-agnostic and has zero UI dependencies.

pub mod node;
pub mod evaluator;
pub mod cfr;
pub mod exploitability;
pub mod test_tree;
pub mod equity;

pub use evaluator::{CactusKevEvaluator, benchmark_throughput};
pub use node::HandEvaluator;
pub use cfr::{CfrSolver, RegretStorage};
pub use exploitability::{compute_exploitability, compute_exploitability_with_evs, ConvergenceMetrics};
pub use test_tree::{build_test_tree, build_test_tree_chance, terminal_ev_table_chance};
pub use equity::{
    hand_equity_ip, terminal_ev_ip_showdown, terminal_ev_ip_fold,
    enumerate_combos, range_ev_ip, build_ev_table_from_eval,
};
