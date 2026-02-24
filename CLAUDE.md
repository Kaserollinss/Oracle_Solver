# Oracle Solver

## Project Overview

Heads-up postflop GTO poker solver targeting Apple Silicon Macs. Computes Nash equilibrium strategies via CFR+ (Counterfactual Regret Minimization Plus). Planned SwiftUI frontend. Currently Phase 2 complete (CFR+ engine on test trees); Phase 3 (tree builder) is next.

Competes with PioSOLVER, GTO+, Simple Postflop. All financial quantities are in big blind units (f64).

See `PRD.txt` for the full 8-phase roadmap.

---

## Development Checklist

### ✅ PHASE 0 — Architecture & Planning
- [x] Memory layout design (`docs/MEMORY_LAYOUT.md`)
- [x] Node struct definitions (`engine/src/node.rs`) — `Card`, `HandRank`, `Node` enum, `GameTree`
- [x] Benchmark targets doc (`docs/BENCHMARKS.md`)
- [x] Exploitability measurement design (`docs/EXPLOITABILITY.md`)
- [x] Architectural patterns doc (`.claude/docs/architectural_patterns.md`)

### ✅ PHASE 1 — Hand Evaluator
- [x] Bitboard representation with suit masks + rank counts
- [x] Flush lookup table (`FLUSH_TABLE: OnceLock<[u16; 8192]>`)
- [x] Non-flush prime-product lookup
- [x] 7-card evaluator (`evaluate_7cards` — single-pass, no heap)
- [x] NEON batch evaluator (`evaluate_batch`, aarch64-gated)
- [x] Criterion benchmark harness (`engine/benches/hand_evaluator.rs`)
- [x] CLI `bench evaluator` command
- [x] Correctness: all hand rank range invariants validated (see MEMORY.md)
- [x] **Batch path verified ≥50M evals/sec** — Rayon-parallel batch: ~89M evals/sec on M-series; scalar: ~16M evals/sec (5.6× speedup)

### ✅ PHASE 2 — CFR+ Engine (on test trees)
- [x] `RegretStorage` — regrets + strategy_sums, indexed by node ID
- [x] Regret matching+ (`current_strategy` — positive-part normalization)
- [x] Linear-weighted strategy accumulation (`accumulate_strategy`)
- [x] CFR+ regret update with floor-at-zero (`update_regrets`)
- [x] `average_strategy` for convergence output
- [x] Pure functional CFR traversal (`cfr_traverse_fn`) — shared `&` refs only
- [x] Rayon parallel Chance-node traversal
- [x] `CfrSolver` wrapper (`run_iteration`)
- [x] Exploitability calculation via best-response traversal (`exploitability.rs`)
- [x] `rayon::join` for parallel IP/OOP best-response passes
- [x] `ConvergenceMetrics` struct matching design doc
- [x] Hardcoded 9-node test tree (`test_tree.rs` — `build_test_tree`)
- [x] Hardcoded 11-node chance test tree (`build_test_tree_chance`)
- [x] CLI `oracle solve` command with `--iterations`, `--threshold`, `--check-every`, `--time-cap`
- [x] Convergence tests (exploitability < 0.01 after 10k iters on test tree)
- [x] **Evaluator integration** — `equity.rs`: `hand_equity_ip`, `terminal_ev_ip_showdown/fold`, `build_ev_table_from_eval` wired to `CactusKevEvaluator`
- [x] **Real range iteration** — `equity.rs`: `enumerate_combos` (dead-card aware), `range_ev_ip` (averages EV over all non-conflicting IP×OOP combos)
- [x] `solver_performance.rs` Criterion benchmark — `cfr_single_iteration`, `cfr_1000_iterations`, `exploitability_check`

### 🔲 PHASE 3 — Tree Builder
- [ ] `GameConfig` struct (board, ranges, bet sizes, stack depth)
- [ ] Action generator — fold/check/call/bet enumeration per street
- [ ] Flop → turn → river node generation
- [ ] Chance nodes for turn/river card dealing (enumerate remaining deck)
- [ ] Terminal node creation with correct pot/stack accounting
- [ ] Connect generated tree to `CfrSolver` + real hand evaluator for terminal EVs
- [ ] End-to-end solve: tree build → CFR+ → convergence report
- [ ] Memory profiling (target: ~31 MB for 100k-node tree per `MEMORY_LAYOUT.md`)
- [ ] Replace `tree/src/lib.rs` stub `build_tree()` with real implementation
- [ ] Add `solver_performance.rs` Criterion benchmark for 100k-node tree

### 🔲 PHASE 4 — Mac UI (MVP Launch)
- [ ] SwiftUI project scaffold (`ui-mac/`)
- [ ] IPC / FFI bridge between Rust engine and Swift
- [ ] Solve control panel (board input, range input, bet sizes, start/stop)
- [ ] 13×13 range heatmap rendering
- [ ] Node navigation (click through tree)
- [ ] EV display per action
- [ ] Frequency visualization (action % per hand combo)
- [ ] IP vs OOP strategy toggle
- [ ] Exploitability progress display during solve

### 🔲 PHASE 5 — Core Feature Expansion
- [ ] Rake modeling (capped/uncapped)
- [ ] Asymmetric stack support
- [ ] Geometric bet sizing
- [ ] Board isomorphism reduction
- [ ] Node locking (fix one player's strategy, re-solve)
- [ ] Strategy export (custom `.oracle` binary format)
- [ ] Batch solving (multiple trees in parallel)
- [ ] Aggregation engine (reports across boards/runouts)

### 🔲 PHASE 6 — Differentiation Layer
- [ ] "Hotness" metric (equity shift by future card)
- [ ] Advanced visualizations
- [ ] Range vs range explorer
- [ ] Blocker impact analysis
- [ ] Drill mode prototype

### 🔲 PHASE 7 — Monetization Features
- [ ] GTO Trainer mode
- [ ] EV loss tracking
- [ ] Hand history import
- [ ] Study reports
- [ ] Cloud solve offload

### 🔲 PHASE 8 — Research & High-Performance Extensions
- [ ] GPU acceleration backend
- [ ] WebGPU experimentation
- [ ] Neural CFR experimentation
- [ ] Real-time resolving

---

## Tech Stack

- **Language**: Rust 2021, workspace resolver v2
- **SIMD**: `std::arch::aarch64` NEON intrinsics (Apple Silicon only, gated with `#[cfg(target_arch)]`)
- **Benchmarking**: `criterion` v0.5 with `html_reports` (dev-dependency only)
- **Parallelism**: `rayon` (Chance-node traversal, parallel BR calculation — active)
- **Planned**: DuckDB (analysis queries)
- **Runtime deps**: `rayon` only; no UI/DB deps in engine

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
- `engine/src/cfr.rs` — `CfrSolver`, `RegretStorage`, `cfr_traverse_fn` (Rayon-parallel)
- `engine/src/exploitability.rs` — `compute_exploitability`, best-response traversal, `ConvergenceMetrics`
- `engine/src/test_tree.rs` — 9-node and 11-node hardcoded test trees + fixed EV tables
- `engine/src/lib.rs` — Public API surface (re-exports)
- `cli/src/main.rs` — CLI entry point: `bench evaluator` + `solve` commands

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
