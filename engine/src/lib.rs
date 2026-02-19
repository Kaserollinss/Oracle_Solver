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

pub use evaluator::{CactusKevEvaluator, benchmark_throughput};
pub use node::HandEvaluator;
pub use cfr::{CfrSolver, RegretStorage};
pub use exploitability::{compute_exploitability, ConvergenceMetrics};
pub use test_tree::build_test_tree;
