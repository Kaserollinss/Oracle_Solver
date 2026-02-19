# oracle Solver

High-performance Mac-native postflop GTO solver with advanced visualization and exploitative tooling.

## Phase 1 - Hand Evaluator

This repository is currently in Phase 1, implementing the hand evaluator:

- ✅ Repository structure (monorepo with `engine`, `tree`, `cli` crates)
- ✅ Memory layout design documentation
- ✅ Node struct definitions
- ✅ Hand evaluator implementation (Cactus Kev two-path approach)
- ✅ Benchmark harness for evaluator throughput
- ✅ CLI benchmark tool (`oracle bench evaluator`)
- ✅ Comprehensive tests (chain assertions, large-sample validation)

## Repository Structure

```
oracle-solver/
├── engine/          # Core solver engine (Rust library)
├── tree/            # Tree builder module (Rust library)
├── cli/             # CLI harness (Rust binary)
├── docs/            # Design documentation
└── PRD.txt          # Product Requirements Document
```

## Building

```bash
# Build all crates
cargo build

# Build in release mode
cargo build --release

# Run CLI
cargo run --bin oracle
```

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run hand evaluator benchmark
cargo bench --bench hand_evaluator

# Run memory layout benchmark
cargo bench --bench memory_layout

# Run with output
cargo bench -- --nocapture

# Run CLI benchmark tool
cargo run --release --bin oracle bench evaluator
cargo run --release --bin oracle bench evaluator 10000000  # 10M hands
```

The hand evaluator benchmark measures 7-card evaluation throughput. Target: 50M+ evals/sec on Apple Silicon with NEON batching.

## Documentation

- [Memory Layout Design](docs/MEMORY_LAYOUT.md) - Internal solver memory layout and data structures
- [Exploitability Measurement](docs/EXPLOITABILITY.md) - How exploitability is calculated and used for convergence
- [Benchmark Targets](docs/BENCHMARKS.md) - Performance targets and measurement methodology
- [Product Requirements](PRD.txt) - Full product requirements document

## Development Status

**Current Phase**: Phase 1 - Hand Evaluator ✅

**Next Phase**: Phase 2 - CFR+ Engine (target: <60s for <100k-node tree)

## License

MIT
