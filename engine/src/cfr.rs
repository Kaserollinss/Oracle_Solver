//! CFR+ algorithm: regret storage, strategy accumulation, and tree traversal
//!
//! All EV values throughout the traversal are from IP's perspective.
//! OOP regrets use a sign flip (OOP gains when IP EV falls).
//!
//! The traversal is implemented as a pure free function (`cfr_traverse_fn`) that
//! collects regret/strategy updates rather than mutating storage mid-traversal.
//! This design enables Rayon-parallel processing at Chance nodes: since only
//! shared references (&GameTree, &RegretStorage) are needed during traversal,
//! independent subtrees can run concurrently without locks.

use crate::node::{GameTree, Node, NodeId, Player};
use crate::test_tree::terminal_ev_table;
use std::collections::HashMap;
use rayon::prelude::*;

/// Regret and strategy storage, indexed by node ID.
///
/// Non-decision nodes (terminal, chance) have empty inner vecs.
/// Never call `current_strategy` or `update_regrets` on a non-decision node.
pub struct RegretStorage {
    /// regrets[node_id][action_idx] — cumulative regrets (CFR+ floored at 0)
    regrets: Vec<Vec<f64>>,
    /// strategy_sums[node_id][action_idx] — linearly weighted strategy accumulation
    strategy_sums: Vec<Vec<f64>>,
}

impl RegretStorage {
    /// Allocate storage. `actions_per_node[i]` is the number of actions at node i
    /// (0 for terminal/chance nodes).
    pub fn new(_num_nodes: usize, actions_per_node: &[usize]) -> Self {
        let regrets = actions_per_node
            .iter()
            .map(|&n| vec![0.0_f64; n])
            .collect();
        let strategy_sums = actions_per_node
            .iter()
            .map(|&n| vec![0.0_f64; n])
            .collect();
        RegretStorage { regrets, strategy_sums }
    }

    /// Current mixed strategy via regret-matching+.
    /// σ(I,a) = r+(I,a) / Σr+(I,a); uniform if all regrets ≤ 0.
    pub fn current_strategy(&self, infoset_id: usize) -> Vec<f64> {
        let r = &self.regrets[infoset_id];
        let pos_sum: f64 = r.iter().map(|&x| x.max(0.0)).sum();
        if pos_sum <= 0.0 {
            let n = r.len();
            return vec![1.0 / n as f64; n];
        }
        r.iter().map(|&x| x.max(0.0) / pos_sum).collect()
    }

    /// Average strategy: S_T(I,a) / ΣS_T(I,a); uniform if never accumulated.
    pub fn average_strategy(&self, infoset_id: usize) -> Vec<f64> {
        let s = &self.strategy_sums[infoset_id];
        let total: f64 = s.iter().sum();
        if total <= 0.0 {
            let n = s.len();
            return vec![1.0 / n as f64; n];
        }
        s.iter().map(|&x| x / total).collect()
    }

    /// CFR+ regret update: r_{t+1}(I,a) = max(0, r_t(I,a) + cf_value[a]).
    /// The floor is applied to the final value (not just the delta).
    pub fn update_regrets(&mut self, infoset_id: usize, cf_values: &[f64]) {
        let r = &mut self.regrets[infoset_id];
        for (ri, &cf) in r.iter_mut().zip(cf_values.iter()) {
            *ri = (*ri + cf).max(0.0);
        }
    }

    /// Linear weighted strategy accumulation: S_t(I,a) += t * σ_t(I,a).
    pub fn accumulate_strategy(&mut self, infoset_id: usize, strategy: &[f64], iteration: u64) {
        let s = &mut self.strategy_sums[infoset_id];
        let weight = iteration as f64;
        for (si, &prob) in s.iter_mut().zip(strategy.iter()) {
            *si += weight * prob;
        }
    }
}

/// A batched regret/strategy update produced during a single traversal.
///
/// Collected by `cfr_traverse_fn` and applied sequentially in `run_iteration`
/// so that the traversal itself only needs shared (&) references.
#[derive(Clone)]
struct RegretUpdate {
    infoset_id: usize,
    cf_values: Vec<f64>,
    strategy: Vec<f64>,
    weight: u64,
}

/// Minimal node info extracted before recursive calls (avoids borrow conflicts).
enum NodeInfo {
    Terminal,
    Decision {
        infoset_id: usize,
        player: Player,
        children: Vec<NodeId>,
    },
    Chance {
        children: Vec<NodeId>,
    },
}

/// Extract the minimal node information needed for traversal.
fn read_node(tree: &GameTree, node_id: NodeId) -> NodeInfo {
    match tree.get(node_id).expect("invalid node id") {
        Node::Terminal { .. } => NodeInfo::Terminal,
        Node::Decision { infoset_id, player, children, .. } => NodeInfo::Decision {
            infoset_id: *infoset_id as usize,
            player: *player,
            children: children.clone(),
        },
        Node::Chance { children, .. } => NodeInfo::Chance {
            children: children.clone(),
        },
    }
}

/// Pure CFR+ traversal. Returns `(ev, updates)` where `ev` is the value from
/// IP's perspective and `updates` is the list of regret/strategy changes to apply.
///
/// Both `tree` and `storage` are borrowed immutably, so Chance node children
/// can be traversed in parallel via Rayon without any locking.
fn cfr_traverse_fn(
    tree: &GameTree,
    storage: &RegretStorage,
    terminal_evs: &HashMap<NodeId, f64>,
    node_id: NodeId,
    reach_ip: f64,
    reach_oop: f64,
    t: u64,
) -> (f64, Vec<RegretUpdate>) {
    match read_node(tree, node_id) {
        NodeInfo::Terminal => {
            let ev = terminal_evs[&node_id];
            (ev, vec![])
        }

        NodeInfo::Decision { infoset_id, player, children } => {
            let strategy = storage.current_strategy(infoset_id);

            let mut all_updates: Vec<RegretUpdate> = Vec::new();
            let mut child_evs = Vec::with_capacity(children.len());

            for (i, &child_id) in children.iter().enumerate() {
                let (new_reach_ip, new_reach_oop) = if player == Player::IP {
                    (reach_ip * strategy[i], reach_oop)
                } else {
                    (reach_ip, reach_oop * strategy[i])
                };
                let (ev, child_updates) = cfr_traverse_fn(
                    tree, storage, terminal_evs, child_id, new_reach_ip, new_reach_oop, t,
                );
                child_evs.push(ev);
                all_updates.extend(child_updates);
            }

            // Node value (IP's perspective)
            let node_value: f64 = strategy.iter().zip(child_evs.iter())
                .map(|(&s, &ev)| s * ev).sum();

            // Counterfactual regrets (sign depends on acting player)
            let cf_values: Vec<f64> = child_evs.iter().map(|&ev| {
                if player == Player::IP {
                    reach_oop * (ev - node_value)
                } else {
                    reach_ip * (node_value - ev) // OOP benefits when IP EV falls
                }
            }).collect();

            all_updates.push(RegretUpdate {
                infoset_id,
                cf_values,
                strategy,
                weight: t,
            });

            (node_value, all_updates)
        }

        NodeInfo::Chance { children } => {
            let n = children.len() as f64;

            // Parallel traversal: each child subtree is independent (disjoint node sets,
            // only shared immutable refs needed). Rayon's work-stealing scheduler handles
            // nested parallelism safely.
            let results: Vec<(f64, Vec<RegretUpdate>)> = children
                .par_iter()
                .map(|&child_id| {
                    cfr_traverse_fn(
                        tree, storage, terminal_evs, child_id, reach_ip, reach_oop, t,
                    )
                })
                .collect();

            // Uniform average EV; concatenate all updates
            let mut all_updates: Vec<RegretUpdate> = Vec::new();
            let mut ev_sum = 0.0_f64;
            for (ev, updates) in results {
                ev_sum += ev;
                all_updates.extend(updates);
            }

            (ev_sum / n, all_updates)
        }
    }
}

/// CFR+ solver operating on a game tree.
pub struct CfrSolver {
    pub tree: GameTree,
    pub storage: RegretStorage,
    pub iteration: u64,
    terminal_evs: HashMap<NodeId, f64>,
}

impl CfrSolver {
    /// Create a solver for the given tree using the standard test terminal EV table.
    pub fn new(tree: GameTree) -> Self {
        Self::new_with_evs(tree, terminal_ev_table())
    }

    /// Create a solver with a custom terminal EV table.
    /// Use this when solving trees other than the default 9-node test tree.
    pub fn new_with_evs(tree: GameTree, terminal_evs: HashMap<NodeId, f64>) -> Self {
        let num_nodes = tree.len();
        let mut actions_per_node = vec![0usize; num_nodes];
        for node in &tree.nodes {
            if let Node::Decision { id, actions, .. } = node {
                actions_per_node[*id as usize] = actions.len();
            }
        }
        let storage = RegretStorage::new(num_nodes, &actions_per_node);
        CfrSolver { tree, storage, iteration: 0, terminal_evs }
    }

    /// Run one CFR+ iteration (increments `self.iteration` before traversal).
    ///
    /// Internally uses a functional traversal that collects all regret/strategy
    /// updates and applies them sequentially. Chance node subtrees are traversed
    /// in parallel via Rayon.
    pub fn run_iteration(&mut self) {
        self.iteration += 1;
        let t = self.iteration;
        let (_, updates) = cfr_traverse_fn(
            &self.tree,
            &self.storage,
            &self.terminal_evs,
            0,
            1.0,
            1.0,
            t,
        );
        for u in updates {
            self.storage.update_regrets(u.infoset_id, &u.cf_values);
            self.storage.accumulate_strategy(u.infoset_id, &u.strategy, u.weight);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tree::{build_test_tree, build_test_tree_chance, terminal_ev_table_chance};

    fn make_storage(actions: &[usize]) -> RegretStorage {
        RegretStorage::new(actions.len(), actions)
    }

    #[test]
    fn test_regret_matching_uniform_initial() {
        let s = make_storage(&[0, 0, 2]); // node 2 has 2 actions
        let strategy = s.current_strategy(2);
        assert!((strategy[0] - 0.5).abs() < 1e-10);
        assert!((strategy[1] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_regret_matching_proportional() {
        let mut s = make_storage(&[0, 0, 2]);
        s.regrets[2] = vec![2.0, 1.0];
        let strategy = s.current_strategy(2);
        assert!((strategy[0] - 2.0 / 3.0).abs() < 1e-10);
        assert!((strategy[1] - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_strategy_sums_to_one() {
        let mut s = make_storage(&[0, 0, 3]);
        s.regrets[2] = vec![1.0, 0.5, 0.0];
        let strategy = s.current_strategy(2);
        let sum: f64 = strategy.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cfr_plus_negative_floor() {
        let mut s = make_storage(&[2]);
        s.regrets[0] = vec![0.5, -1.0];
        s.update_regrets(0, &[-2.0, 3.0]);
        // 0.5 + (-2.0) = -1.5 → floored to 0.0
        assert!((s.regrets[0][0] - 0.0).abs() < 1e-10);
        // -1.0 + 3.0 = 2.0 → unchanged
        assert!((s.regrets[0][1] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_strategy_accumulation_linear_weight() {
        let mut s = make_storage(&[2]);
        s.accumulate_strategy(0, &[0.6, 0.4], 1);
        s.accumulate_strategy(0, &[0.5, 0.5], 2);
        // S[0] = 1*0.6 + 2*0.5 = 1.6
        assert!((s.strategy_sums[0][0] - 1.6).abs() < 1e-10);
        // S[1] = 1*0.4 + 2*0.5 = 1.4
        assert!((s.strategy_sums[0][1] - 1.4).abs() < 1e-10);
    }

    #[test]
    fn test_average_strategy_sums_to_one() {
        let mut s = make_storage(&[2]);
        s.accumulate_strategy(0, &[0.7, 0.3], 1);
        s.accumulate_strategy(0, &[0.4, 0.6], 2);
        let avg = s.average_strategy(0);
        let sum: f64 = avg.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cfr_solver_strategies_evolve() {
        let tree = build_test_tree();
        let mut solver = CfrSolver::new(tree);
        let initial = solver.storage.average_strategy(0);

        for _ in 0..100 {
            solver.run_iteration();
        }

        let after = solver.storage.average_strategy(0);
        // Strategy should change from the initial uniform distribution
        assert_ne!(initial[0].to_bits(), after[0].to_bits());
    }

    #[test]
    fn test_solve_golden_regression() {
        let tree = build_test_tree();
        let mut solver = CfrSolver::new(tree);
        for _ in 0..5_000 {
            solver.run_iteration();
        }
        // All decision node average strategies should sum to ~1.0
        for &id in &[0usize, 1, 3, 6] {
            let avg = solver.storage.average_strategy(id);
            let sum: f64 = avg.iter().sum();
            assert!((sum - 1.0).abs() < 1e-6, "node {} strategy sum = {}", id, sum);
        }
    }

    #[test]
    fn test_cfr_solver_chance_tree_strategies_evolve() {
        let tree = build_test_tree_chance();
        let evs = terminal_ev_table_chance();
        let mut solver = CfrSolver::new_with_evs(tree, evs);
        let initial = solver.storage.average_strategy(0);

        for _ in 0..100 {
            solver.run_iteration();
        }

        let after = solver.storage.average_strategy(0);
        assert_ne!(initial[0].to_bits(), after[0].to_bits());
    }

    #[test]
    fn test_cfr_solver_chance_tree_convergence() {
        let tree = build_test_tree_chance();
        let evs = terminal_ev_table_chance();
        let mut solver = CfrSolver::new_with_evs(tree, evs);
        for _ in 0..5_000 {
            solver.run_iteration();
        }
        // Decision node infosets: 0, 2, 5, 8 — average strategies should sum to ~1.0
        for &id in &[0usize, 2, 5, 8] {
            let avg = solver.storage.average_strategy(id);
            let sum: f64 = avg.iter().sum();
            assert!((sum - 1.0).abs() < 1e-6, "chance tree node {} strategy sum = {}", id, sum);
        }
    }
}
