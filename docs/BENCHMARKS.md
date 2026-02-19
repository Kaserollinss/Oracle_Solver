# Benchmark Targets and Measurement Design

This document defines performance targets and benchmark methodology for oracle Solver.

## Performance Targets

### Hand Evaluator (Phase 1)

**Target**: ≥50M 7-card evaluations per second on Apple Silicon

#### Justification for M2 Mac Air

The M2 chip provides:
- **CPU throughput**: ~200 GFLOPS (theoretical peak)
- **Cache hierarchy**: Excellent L1/L2/L3 cache with high bandwidth
- **SIMD support**: NEON instructions for vectorization

A well-optimized 7-card evaluator using lookup tables and bitwise operations requires approximately **50-100 operations per evaluation**:
- Bitboard operations: ~10-20 ops
- Lookup table accesses: ~5-10 ops
- Comparison and ranking: ~10-20 ops
- Memory access overhead: ~25-50 ops

At 50M evals/sec with 50-100 ops/eval:
- **Operations/sec**: 2.5-5 billion ops/sec
- **Percentage of peak**: ~1.25-2.5% of theoretical peak

**Lookup table arithmetic**: Each 7-card evaluation requires 21 lookups (one per 5-card subset). At 50M evals/sec, that's 21 × 50M = **1.05 billion lookups/sec**. This arithmetic grounds the target—whether it's achievable depends on lookup table cache behavior and SIMD batching, but the math is explicit.

This is realistic given:
- SIMD (NEON) acceleration can process multiple evaluations in parallel
- Cache-friendly lookup patterns minimize memory latency
- Modern bitboard evaluators (e.g., Two Plus Two evaluator) achieve similar throughput on comparable hardware
- Real-world implementations achieve 30-80M evals/sec on M2-class hardware

**Note**: The scalar path will be benchmarked first to validate correctness. The 50M evals/sec target assumes NEON batching—scalar performance will be lower (typically 10-30M evals/sec) and is acceptable for correctness validation, but the batch API with NEON is required to reach the Phase 1 performance target.

#### Measurement Methodology

- **Batch evaluation**: Evaluate large batches (1M+ hands) to amortize overhead
- **Warm-up**: Run 10k evaluations to warm caches before timing
- **Multiple runs**: Take median of 10 runs to account for system variance
- **Release builds**: Use `cargo build --release` with optimizations
- **Environment**: M2 Mac Air (or M1/M2/M3 equivalent), no other heavy processes

### Solver Performance (Phase 2+)

**Target**: Solve small flop tree (<100k nodes) in <60 seconds on Mac

#### Justification

A 100k-node tree with CFR+ requires:

**Per iteration**:
- ~100k regret updates (one per decision node)
- ~100k strategy accumulations
- ~100k reach probability calculations
- Tree traversal (forward + backward pass)

With efficient memory layout (flat arrays, cache-friendly traversal):
- **Per iteration time**: <1ms (realistic with optimized Rust)
- **Memory bandwidth**: M2 has ~100 GB/s, easily supports this workload

**Convergence**:
- Typical convergence: 10k-100k iterations depending on exploitability threshold
- At 0.1% pot threshold: ~20k-50k iterations typical
- At 10k iterations: ~10 seconds total
- At 50k iterations: ~50 seconds total
- At 100k iterations: ~100 seconds total

The **<60s target** assumes convergence around **20k-50k iterations**, which is realistic for:
- Standard flop trees (2 bet sizes per street, reasonable branching)
- 0.1% pot exploitability threshold
- Well-optimized CFR+ implementation

#### What "Small Flop Tree" Means

For benchmarking purposes, a "small flop tree" is defined as:
- **Max nodes**: <100,000
- **Bet sizes**: 2 sizes per street (e.g., 33% pot, 75% pot)
- **Streets**: Flop → Turn → River (3 streets)
- **Stack depth**: 100bb
- **No rake**: Simplifies calculation

This represents a typical "standard" solve that users would run frequently.

#### Measurement Methodology

- **Fixed tree**: Use a canonical test tree (same structure every run)
- **Deterministic**: Use fixed random seed for reproducibility
- **Convergence target**: 0.1% pot exploitability
- **Timing**: Measure from solve start to convergence (or iteration cap)
- **Environment**: M2 Mac Air, release builds, single-threaded initially, then multi-threaded

### Exploitability Precision

**Target**: 0.1% pot precision

For a 100bb pot:
- 0.1% pot = 0.1 bb
- This is sufficient for practical GTO analysis
- More precise thresholds (0.01% pot) may take significantly longer

#### Measurement

Exploitability is measured as:
- **Per-infoset**: Not used (too granular)
- **Full-tree**: Sum of both players' best-response gains (standard definition)

See [EXPLOITABILITY.md](EXPLOITABILITY.md) for details.

## Benchmark Environment

### Hardware

- **Primary target**: Apple Silicon (M1/M2/M3)
- **Specific baseline**: M2 Mac Air (8-core CPU, unified memory)
- **Memory**: 16GB+ recommended for larger trees

### Software

- **Rust version**: Latest stable (1.70+)
- **Build flags**: `cargo build --release` with default optimizations
- **OS**: macOS (latest)
- **No background processes**: Close other applications during benchmarking

### Measurement Approach

- **Single run**: Initial development/debugging
- **Median of N runs**: Production benchmarks (N=10 recommended)
- **Warm-up**: Run benchmarks once before timing to warm caches
- **Statistical reporting**: Report min, median, max, and standard deviation

## Benchmark Harness

### Structure

Benchmarks are organized in `engine/benches/` directory using Cargo's built-in benchmark support:

```
engine/
  benches/
    hand_evaluator.rs    # Phase 1: Evaluator throughput
    solver_performance.rs # Phase 2: Full solve timing
    memory_layout.rs     # Phase 0: Placeholder benchmark
```

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench hand_evaluator

# Run with output
cargo bench -- --nocapture
```

### Placeholder Benchmark (Phase 0)

A minimal benchmark exists to validate the benchmark pipeline:

- **Name**: `solver_memory_layout_iteration`
- **Purpose**: Iterate over a small `Vec<Node>` to validate benchmark setup
- **Target**: <1ms for 10k node iteration (trivial, just validates infrastructure)

## Future Benchmarks

### Phase 1 Benchmarks

- **Hand evaluator throughput**: 7-card eval/sec
- **Batch evaluation**: Evaluate 1M hands, measure time
- **Hand evaluator throughput**: 7-card eval/sec (scalar path benchmarked first, then batch + NEON)
- **Batch evaluation**: Evaluate 1M hands, measure time
- **Cache performance**: Measure cache hit rates

### Phase 2 Benchmarks

- **CFR+ iteration time**: Single iteration over 100k-node tree
- **Convergence time**: Time to reach 0.1% pot exploitability
- **Memory usage**: Peak memory during solve
- **Multi-threading scaling**: Speedup with 2/4/8 threads

### Phase 3+ Benchmarks

- **Tree construction time**: Time to build 100k-node tree
- **End-to-end solve**: Tree build + solve + convergence
- **Large tree performance**: 1M+ node trees
- **Batch solving**: Multiple trees in parallel

## Success Criteria

### MVP Success (Phase 4)

- ✅ Hand evaluator: ≥50M evals/sec on M2
- ✅ Solver: <60s for <100k-node tree on M2
- ✅ Exploitability: Accurate to 0.1% pot
- ✅ Stability: Zero crashes during benchmark runs

### Post-MVP Success

- Node locking: No performance regression
- Batch solving: Linear scaling with number of trees
- Large trees: Graceful degradation (not exponential slowdown)
