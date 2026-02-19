# Architectural Patterns

Patterns that appear in multiple files across the codebase.

## Trait-Based Abstraction for Swappable Implementations

`HandEvaluator` trait (`engine/src/node.rs:48-54`) defines the single-method interface. `CactusKevEvaluator` implements it (`engine/src/evaluator.rs:128-132`). Call sites depend on the trait, not the concrete type — future evaluators (e.g., lookup-table-only, GPU) can be swapped without changing consumers.

## `Default` Delegates to `new()`

Both `CactusKevEvaluator` (`engine/src/evaluator.rs:122-126`) and `GameTree` (`engine/src/node.rs:282-286`) implement `Default` by calling `Self::new()`. Use `new()` as the canonical constructor; `Default` is provided for ergonomic compatibility with stdlib and frameworks.

## Newtype Wrappers for Semantic Primitives

`Card(u8)` and `HandRank(u16)` (`engine/src/node.rs:9-42`) prevent raw integers from being passed where semantic types are expected. Add new newtypes for any new primitive with domain meaning (e.g., `InfosetId`, `NodeId` are `u32` type aliases at `engine/src/node.rs:100-105`).

## Enum-Based Sum Types with Exhaustive `match` Accessors

`Node` (`engine/src/node.rs:113-174`) is a three-variant enum (Decision, Chance, Terminal). Shared fields like `id()` are exposed via methods that exhaustively match all variants (`engine/src/node.rs:176-244`). Predicate helpers use the `matches!` macro (`engine/src/node.rs:231-243`). When adding new shared accessors, always cover all variants.

## Architecture-Conditional Code via `#[cfg(target_arch)]`

SIMD paths are gated at compile time, never at runtime. `evaluate_batch` (`engine/src/evaluator.rs:103-119`) branches on `target_arch = "aarch64"` — the NEON module (`engine/src/evaluator.rs:353-472`) is compiled only on ARM. A matching `#[cfg(not(target_arch = "aarch64"))]` stub keeps non-ARM builds valid. Follow this pattern for any platform-specific code.

## Private Sub-Module for Implementation Details

`mod tables` (`engine/src/evaluator.rs:182-350`) contains lookup functions and helpers with `pub(crate)` visibility. The public-facing evaluator API is kept clean; internal math is isolated and testable separately. Use private modules to encapsulate subsystems within a single file.

## Flat Array Storage with Index-Based Relationships

`GameTree` stores all nodes in `Vec<Node>` (`engine/src/node.rs:249-286`) indexed by `NodeId` (a `u32`). Parent/child links are stored as integer indices, not pointers. This eliminates pointer chasing and enables cache-friendly traversal and SIMD. The same pattern is planned for `RegretStorage` (documented in `docs/MEMORY_LAYOUT.md:60-77`): solver state lives in separate parallel arrays, not inside the tree nodes.

## Separation of Immutable Tree Structure from Mutable Solver State

`Node` types hold only game-state data (pot, stacks, board, actions). Regrets and cumulative strategies are stored in separate arrays indexed by `InfosetId` — not embedded in tree nodes. This allows tree reuse across multiple solves and cache-efficient regret updates. See `docs/MEMORY_LAYOUT.md` for the planned `RegretStorage` layout.

## Crate Root Re-Exports for a Clean Public API

`engine/src/lib.rs:1-12` declares modules (`pub mod node; pub mod evaluator;`) then re-exports the key types with `pub use`. Consumers import from the crate root, not from internal module paths. When adding new public types, add the re-export to `lib.rs`.

## Inline `#[cfg(test)]` Modules

Unit tests live in the same file as the code under test (`engine/src/evaluator.rs:474-705`). Tests use descriptive names that state what property is verified (e.g., `test_hand_rank_ordering`, `test_consistency`). Randomized tests use the project-wide LCG (see below) for reproducibility.

## Deterministic Pseudo-Randomness via LCG (No External Crate)

The same LCG (`state = (state.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff`) appears in both the benchmark harness (`engine/benches/hand_evaluator.rs:11-28`) and the `benchmark_throughput` helper (`engine/src/evaluator.rs:143-146`). Avoids a `rand` dependency while keeping test and benchmark data reproducible and seed-controlled.

## Workspace-Level Metadata Inheritance

`engine/Cargo.toml`, `tree/Cargo.toml`, and `cli/Cargo.toml` all use `version.workspace = true`, `edition.workspace = true`, etc. All shared metadata is set once in the root `Cargo.toml`. When adding a new crate to the workspace, inherit from root rather than duplicating values.

## Two-Path Evaluation (Flush vs. Non-Flush)

`CactusKevEvaluator::evaluate_5cards` (`engine/src/evaluator.rs:62-96`) takes one of two paths:
- **Flush**: Build per-suit rank bitmasks; if any has 5+ bits, look up via `lookup_flush_rank(rank_mask)`.
- **Non-flush**: Multiply per-rank prime numbers (`RANK_PRIMES`); the product uniquely identifies the rank multiset; look up via `lookup_nonflush_rank(product)`.

The prime-product hash is a mathematical invariant (unique factorization), not a heuristic. Both paths return a `u16` rank in 1–7462 scale.
