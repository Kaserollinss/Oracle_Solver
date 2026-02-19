# Internal Solver Memory Layout Design

This document describes the memory layout and data structures used by the oracle Solver engine for efficient CFR+ computation.

## Design Principles

1. **Cache Locality**: Nodes are stored in flat arrays for sequential access patterns
2. **Immutable Tree**: Game tree structure is immutable; solver state is separate
3. **Deterministic Iteration**: Flat storage enables deterministic traversal order
4. **Memory Efficiency**: Parallel arrays indexed by InfosetId minimize memory overhead

## Node Storage

### Flat Array Storage

Nodes are stored in a contiguous `Vec<Node>` array, indexed by `NodeId` (which is a `u32` index). This provides:

- **Cache locality**: Sequential traversal hits L1/L2 cache efficiently
- **Predictable access**: No pointer chasing, enabling SIMD optimizations
- **Memory efficiency**: No per-node pointer overhead (8 bytes saved per node)

The `GameTree` struct wraps this array and provides access methods.

### Node Types

Three node types exist (see `engine/src/node.rs`):

1. **DecisionNode**: Player action required
2. **ChanceNode**: Board card dealt
3. **TerminalNode**: Hand ends (fold or showdown)

All nodes share common fields:
- `id: NodeId` - Index into the flat array
- `parent: Option<NodeId>` - Parent node index (None for root)
- `children: Vec<NodeId>` - Child node indices

### Tree Construction

The `tree` crate builds nodes and populates the `Vec<Node>`. Nodes are constructed in depth-first order, ensuring parent nodes appear before children in the array (though this is not strictly required).

## Information Sets

### Heads-Up Perfect Recall Mapping

In heads-up postflop poker with perfect recall, each node maps **1:1 to an information set**. This invariant holds because:

- Both players see all board cards
- Both players know the action sequence
- No hidden information beyond hole cards (which are handled via terminal EV calculation)

Therefore, `NodeId == InfosetId` for decision nodes. The `infoset_id` field in `DecisionNode` is redundant but kept for clarity and future extensibility.

### Information Set Indexing

Information sets are indexed by `InfosetId` (also `u32`). For decision nodes:
- `infoset_id = node.id`
- Regret and strategy arrays are indexed by `infoset_id`

## Regret and Strategy Storage

### Separation from Tree Structure

**Critical Design Decision**: Regrets and strategies are stored in **separate parallel arrays**, not within Node structs. This enables:

- **Tree reuse**: Same tree can be solved multiple times with different initializations
- **Memory efficiency**: Only decision nodes need regret/strategy storage
- **Cache efficiency**: Regret updates can iterate over dense arrays without touching tree structure

### Storage Layout

```
RegretStorage {
    // Indexed by InfosetId
    regrets: Vec<Vec<f64>>,  // regrets[infoset_id][action_index] = regret
    strategy_sums: Vec<Vec<f64>>,  // strategy_sums[infoset_id][action_index] = sum
    reach_probs: Vec<f64>,  // reach_probs[infoset_id] = reach probability
}
```

For a tree with N decision nodes:
- `regrets`: N × A entries (A = max actions per node, typically 2-4)
- `strategy_sums`: N × A entries
- `reach_probs`: N entries

**Memory Estimate**: For 100k nodes with avg 3 actions:
- Regrets: 100k × 3 × 8 bytes = 2.4 MB
- Strategy sums: 100k × 3 × 8 bytes = 2.4 MB
- Reach probs: 100k × 8 bytes = 0.8 MB
- **Total solver state**: ~5.6 MB per player (×2 for both players = ~11 MB)

Tree structure itself: ~200 bytes per node × 100k = ~20 MB

## Action Indexing

### Action Enumeration

Actions are enumerated as:
- `0`: Fold (if available)
- `1`: Check/Call (depending on context)
- `2`: Bet size 1
- `3`: Bet size 2
- etc.

The `actions: Vec<Action>` field in `DecisionNode` defines the available actions at that node. The index into this vector corresponds to the index into the regret/strategy arrays.

### Child Mapping

The `children: Vec<NodeId>` array is parallel to `actions`:
- `children[i]` is the node reached by taking `actions[i]`
- Terminal nodes have empty `children` arrays

## Player and Street Encoding

### Player

Stored as `Player` enum (`IP` or `OOP`) in decision nodes. The opponent is computed via `player.opponent()`.

### Street

Stored as `Street` enum (`Flop`, `Turn`, `River`) in decision and chance nodes. Terminal nodes don't have a street (game is over).

## Memory Scaling Estimates

### Per-Node Memory

**Decision Node**: ~200 bytes
- Node metadata: 32 bytes
- Actions vector: ~24 bytes + (action_size × num_actions)
- Children vector: ~24 bytes + (4 bytes × num_actions)
- Board cards: ~24 bytes + (1 byte × num_cards)
- Bet sequence: ~24 bytes + (action_size × sequence_length)
- Pot/stacks: 24 bytes

**Chance Node**: ~150 bytes (similar, no actions)

**Terminal Node**: ~180 bytes (similar, includes hole cards)

### Tree Scaling

For a typical flop tree with 100k nodes:
- Tree structure: ~20 MB
- Solver state (regrets/strategies): ~11 MB
- **Total**: ~31 MB

For 1M nodes:
- Tree structure: ~200 MB
- Solver state: ~110 MB
- **Total**: ~310 MB

These estimates assume average branching factor of 3 and typical action sequences. Actual memory usage will vary based on tree structure.

## Traversal Patterns

### CFR+ Iteration

CFR+ requires:
1. **Forward pass**: Traverse tree, compute reach probabilities
2. **Backward pass**: Traverse tree, update regrets

Both passes benefit from flat array storage:
- Sequential access patterns
- Predictable memory access
- Cache-friendly iteration

### Exploitability Calculation

Best-response calculation requires:
1. Traverse tree forward
2. At each decision node, choose best action
3. Accumulate EV

Flat storage enables efficient traversal without pointer chasing.

## Future Optimizations

Potential optimizations for later phases:
- **Arena allocation**: Use bump allocator for nodes to reduce allocations
- **SIMD regret updates**: Vectorize regret updates for multiple actions
- **Memory pooling**: Reuse arrays across solves
- **Compression**: Compress bet sequences for deep trees
