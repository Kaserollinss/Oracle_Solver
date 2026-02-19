# oracle Solver

High-performance Mac-native postflop GTO solver with advanced visualization and exploitative tooling.

## Phase 0 - Architecture & Planning

This repository is currently in Phase 0, which establishes the architecture and planning artifacts:

- ✅ Repository structure (monorepo with `engine`, `tree`, `cli` crates)
- ✅ Memory layout design documentation
- ✅ Node struct definitions
- ✅ Exploitability measurement design
- ✅ Benchmark targets and methodology
- ✅ Placeholder benchmark harness

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

# Run specific benchmark
cargo bench --bench memory_layout

# Run with output
cargo bench -- --nocapture
```

The placeholder benchmark (`memory_layout`) validates the benchmark infrastructure by iterating over a small `Vec<Node>` structure. Real benchmarks will be added in Phase 1 (hand evaluator) and Phase 2 (CFR+ solver).

## Documentation

- [Memory Layout Design](docs/MEMORY_LAYOUT.md) - Internal solver memory layout and data structures
- [Exploitability Measurement](docs/EXPLOITABILITY.md) - How exploitability is calculated and used for convergence
- [Benchmark Targets](docs/BENCHMARKS.md) - Performance targets and measurement methodology
- [Product Requirements](PRD.txt) - Full product requirements document

## Development Status

**Current Phase**: Phase 0 - Architecture & Planning ✅

**Next Phase**: Phase 1 - Hand Evaluator (target: 50M+ 7-card evals/sec)

## License

MIT
