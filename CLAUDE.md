# Oracle Solver

## Project Overview

Heads-up postflop GTO poker solver targeting Apple Silicon Macs. Computes Nash equilibrium strategies via CFR+ (Counterfactual Regret Minimization Plus). Planned SwiftUI frontend. Currently Phase 1 complete (hand evaluator); CFR+ engine is next.

Competes with PioSOLVER, GTO+, Simple Postflop. All financial quantities are in big blind units (f64).

See `PRD.txt` for the full 8-phase roadmap.

## Tech Stack

- **Language**: Rust 2021, workspace resolver v2
- **SIMD**: `std::arch::aarch64` NEON intrinsics (Apple Silicon only, gated with `#[cfg(target_arch)]`)
- **Benchmarking**: `criterion` v0.5 with `html_reports` (dev-dependency only)
- **Planned**: `rayon` (parallelism), DuckDB (analysis queries)
- **No runtime dependencies** in current core logic

## Key Directories

| Path | Purpose |
|---|---|
| `engine/src/` | Core library: card types, hand evaluator, game tree nodes, GameTree struct |
| `engine/benches/` | Criterion benchmarks: evaluator throughput, memory layout |
| `tree/src/` | Game tree builder (Phase 3 stub — `build_tree()` returns empty vec) |
| `cli/src/` | Binary driver: `oracle bench evaluator [N]` command |
| `docs/` | Design docs: memory layout, exploitability algorithm, benchmark targets |

### Key Files

- `engine/src/node.rs` — `Card`, `HandRank`, `HandEvaluator` trait, `Node` enum, `GameTree`
- `engine/src/evaluator.rs` — `CactusKevEvaluator`, NEON batch eval, lookup tables, tests
- `engine/src/lib.rs` — Public API surface (re-exports)
- `cli/src/main.rs` — CLI entry point, argument parsing

## Build & Test Commands

```bash
# Build
cargo build                                       # debug
cargo build --release                             # required for perf work

# Test
cargo test                                        # all unit tests

# Run CLI
cargo run --bin oracle                            # help
cargo run --release --bin oracle bench evaluator  # 1M hand benchmark
cargo run --release --bin oracle bench evaluator 10000000

# Criterion benchmarks
cargo bench                                       # all
cargo bench --bench hand_evaluator
cargo bench --bench memory_layout
```

## Git Workflow

Always work on a branch — never commit directly to `main`.

```bash
# Start any feature or bug fix
git checkout -b feature/<short-description>   # new feature
git checkout -b fix/<short-description>       # bug fix

# Return to main when done
git checkout main
git merge <branch-name>
```

Branch naming: use `feature/` or `fix/` prefix with a short kebab-case description (e.g., `feature/cfr-engine`, `fix/flush-rank-off-by-one`).

### Pre-commit verification (mandatory)

Before committing on any branch, run:

```bash
cargo build && cargo test
```

- If either fails, **do not commit** — fix the error immediately before proceeding.
- Keep iterating until `cargo build && cargo test` passes cleanly, then commit.

## Conventions to Know

- `HandRank`: lower value = stronger hand (Royal Flush = 1, worst High Card = 7462) — `engine/src/node.rs:28`
- Player indexing: `[IP, OOP]` (index 0 = in-position) — `engine/src/node.rs:133`
- `Vec<Card>` for boards (0–5 cards), `[Card; 5]` for 7-card eval input, `[Card; 2]` for hole cards
- Deterministic test/bench data via LCG (no external rand crate) — `engine/src/evaluator.rs:143`

## Additional Documentation

Check these files when working on related areas:

| File | When to consult |
|---|---|
| `.claude/docs/architectural_patterns.md` | Design patterns, conventions, idioms used across the codebase |
| `docs/MEMORY_LAYOUT.md` | Flat array storage, regret/strategy indexing, cache layout |
| `docs/EXPLOITABILITY.md` | Best-response calculation, convergence algorithm |
| `docs/BENCHMARKS.md` | Performance targets, measurement methodology |
| `PRD.txt` | Full product spec, phase breakdown, feature roadmap |
