//! Per-hand CFR+ solver with vector-reach traversal
//!
//! Unlike the scalar `cfr.rs` (single EV per terminal), this module tracks
//! separate regrets and strategies for each hand combo in each player's range.
//! Terminal EVs are computed via the hand evaluator using precomputed rank tables.
//!
//! ## Architecture
//! - Single tree traversal per iteration (not per hand pair)
//! - Reach probabilities are vectors: `reach[combo_idx]` for each player
//! - Mutable traversal: regrets/strategies updated in-place during traversal
//! - Precomputed `RankCache` and conflict table for fast terminal evaluation
//!
//! ## EV convention
//! All values from IP's perspective, in big blinds.

use std::collections::HashMap;

use crate::evaluator::CactusKevEvaluator;
use crate::node::{Card, GameTree, Node, NodeId, Player};

// ─── HandIndexer ─────────────────────────────────────────────────────────────

/// Maps hand combos to contiguous indices for each player.
pub struct HandIndexer {
    /// Valid 2-card combos (excludes initial board cards)
    pub combos: Vec<[Card; 2]>,
}

impl HandIndexer {
    /// Build an indexer from the initial board cards.
    /// Both ranges start as all valid combos; range filtering can come later.
    pub fn new(board: &[Card]) -> Self {
        let combos = crate::equity::enumerate_combos(board);
        HandIndexer { combos }
    }

    pub fn num_combos(&self) -> usize {
        self.combos.len()
    }
}

// ─── RankCache ───────────────────────────────────────────────────────────────

/// Canonicalize a 5-card board to a u64 key for caching.
fn board_key(board: &[Card]) -> u64 {
    let mut vals: [u8; 5] = [
        board[0].value(),
        board[1].value(),
        board[2].value(),
        board[3].value(),
        board[4].value(),
    ];
    vals.sort();
    vals.iter()
        .enumerate()
        .fold(0u64, |acc, (i, &v)| acc | ((v as u64) << (i * 8)))
}

/// Precomputed hand ranks for all combos on unique 5-card boards.
/// `ranks[combo_idx]` = HandRank value, or `u16::MAX` if combo is blocked by board.
struct RankCache {
    cache: HashMap<u64, Vec<u16>>,
}

impl RankCache {
    /// Scan the tree for all showdown terminals, precompute ranks for each unique board.
    fn build(tree: &GameTree, combos: &[[Card; 2]]) -> Self {
        let eval = CactusKevEvaluator::new();
        let mut cache = HashMap::new();

        for node in &tree.nodes {
            if let Node::Terminal {
                board,
                folder: None,
                ..
            } = node
            {
                if board.len() != 5 {
                    continue;
                }
                let key = board_key(board);
                if cache.contains_key(&key) {
                    continue;
                }

                let board5 = [board[0], board[1], board[2], board[3], board[4]];
                let ranks: Vec<u16> = combos
                    .iter()
                    .map(|hand| {
                        // Check if hand conflicts with board
                        if board
                            .iter()
                            .any(|bc| bc.value() == hand[0].value() || bc.value() == hand[1].value())
                        {
                            u16::MAX // sentinel: blocked combo
                        } else {
                            eval.evaluate_7cards(board5, *hand).value()
                        }
                    })
                    .collect();

                cache.insert(key, ranks);
            }
        }
        RankCache { cache }
    }

    fn get(&self, board: &[Card]) -> Option<&Vec<u16>> {
        if board.len() != 5 {
            return None;
        }
        self.cache.get(&board_key(board))
    }

    fn memory_bytes(&self) -> usize {
        self.cache
            .values()
            .map(|v| v.len() * std::mem::size_of::<u16>())
            .sum::<usize>()
            + self.cache.len() * (std::mem::size_of::<u64>() + std::mem::size_of::<Vec<u16>>())
    }
}

// ─── HandRegrets ─────────────────────────────────────────────────────────────

/// Per-hand regret and strategy storage for a single decision node.
///
/// Layout: `data[hand_idx * num_actions + action_idx]`
struct HandRegrets {
    num_actions: usize,
    #[allow(dead_code)]
    num_hands: usize,
    regrets: Vec<f64>,
    strategy_sums: Vec<f64>,
}

impl HandRegrets {
    fn new(num_hands: usize, num_actions: usize) -> Self {
        let len = num_hands * num_actions;
        HandRegrets {
            num_actions,
            num_hands,
            regrets: vec![0.0; len],
            strategy_sums: vec![0.0; len],
        }
    }

    /// Get current strategy for a specific hand via regret matching+.
    fn current_strategy(&self, hand_idx: usize) -> Vec<f64> {
        let start = hand_idx * self.num_actions;
        let r = &self.regrets[start..start + self.num_actions];
        let pos_sum: f64 = r.iter().map(|&x| x.max(0.0)).sum();
        if pos_sum <= 0.0 {
            return vec![1.0 / self.num_actions as f64; self.num_actions];
        }
        r.iter().map(|&x| x.max(0.0) / pos_sum).collect()
    }

    /// CFR+ regret update for a specific hand.
    fn update_regrets(&mut self, hand_idx: usize, cf_values: &[f64]) {
        let start = hand_idx * self.num_actions;
        for (i, &cf) in cf_values.iter().enumerate() {
            let r = &mut self.regrets[start + i];
            *r = (*r + cf).max(0.0);
        }
    }

    /// Linear weighted strategy accumulation for a specific hand.
    fn accumulate_strategy(&mut self, hand_idx: usize, strategy: &[f64], iteration: u64) {
        let start = hand_idx * self.num_actions;
        let weight = iteration as f64;
        for (i, &prob) in strategy.iter().enumerate() {
            self.strategy_sums[start + i] += weight * prob;
        }
    }

    /// Average strategy for a specific hand.
    fn average_strategy(&self, hand_idx: usize) -> Vec<f64> {
        let start = hand_idx * self.num_actions;
        let s = &self.strategy_sums[start..start + self.num_actions];
        let total: f64 = s.iter().sum();
        if total <= 0.0 {
            return vec![1.0 / self.num_actions as f64; self.num_actions];
        }
        s.iter().map(|&x| x / total).collect()
    }

    fn memory_bytes(&self) -> usize {
        (self.regrets.len() + self.strategy_sums.len()) * std::mem::size_of::<f64>()
    }
}

// ─── RangeRegretStorage ──────────────────────────────────────────────────────

/// Storage for per-hand regrets/strategies across the entire tree.
pub struct RangeRegretStorage {
    /// `node_data[node_id]` is `Some(HandRegrets)` for decision nodes, `None` otherwise.
    node_data: Vec<Option<HandRegrets>>,
}

impl RangeRegretStorage {
    /// Build storage for the given tree using a single combo list.
    fn new(tree: &GameTree, num_combos: usize) -> Self {
        let mut node_data = Vec::with_capacity(tree.len());
        for node in &tree.nodes {
            match node {
                Node::Decision { actions, .. } => {
                    node_data.push(Some(HandRegrets::new(num_combos, actions.len())));
                }
                _ => node_data.push(None),
            }
        }
        RangeRegretStorage { node_data }
    }

    fn memory_bytes(&self) -> usize {
        self.node_data
            .iter()
            .map(|opt| match opt {
                Some(hr) => hr.memory_bytes(),
                None => 0,
            })
            .sum()
    }
}

// ─── MemoryReport ────────────────────────────────────────────────────────────

/// Memory usage breakdown for the solver.
pub struct MemoryReport {
    pub tree_bytes: usize,
    pub storage_bytes: usize,
    pub rank_cache_bytes: usize,
    pub conflict_table_bytes: usize,
    pub total_bytes: usize,
}

impl std::fmt::Display for MemoryReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "tree={:.1} MB, storage={:.1} MB, rank_cache={:.1} MB, conflict={:.1} MB, total={:.1} MB",
            self.tree_bytes as f64 / 1_048_576.0,
            self.storage_bytes as f64 / 1_048_576.0,
            self.rank_cache_bytes as f64 / 1_048_576.0,
            self.conflict_table_bytes as f64 / 1_048_576.0,
            self.total_bytes as f64 / 1_048_576.0,
        )
    }
}

// ─── RangeSolver ─────────────────────────────────────────────────────────────

/// Per-hand CFR+ solver with vector-reach traversal.
///
/// Traverses the tree once per iteration, carrying reach probability vectors
/// for all combos. Regrets and strategies are updated in-place during traversal.
pub struct RangeSolver {
    pub tree: GameTree,
    pub indexer: HandIndexer,
    pub storage: RangeRegretStorage,
    pub iteration: u64,
    rank_cache: RankCache,
    /// conflict_table[i * num_combos + j] = true if combos i and j share a card
    conflict_table: Vec<bool>,
}

impl RangeSolver {
    /// Create a new range solver for the given tree and initial board.
    pub fn new(tree: GameTree, board: &[Card]) -> Self {
        let indexer = HandIndexer::new(board);
        let nc = indexer.num_combos();
        let storage = RangeRegretStorage::new(&tree, nc);
        let rank_cache = RankCache::build(&tree, &indexer.combos);

        // Precompute conflict table
        let mut conflict_table = vec![false; nc * nc];
        for i in 0..nc {
            for j in 0..nc {
                let a = &indexer.combos[i];
                let b = &indexer.combos[j];
                conflict_table[i * nc + j] = a[0].value() == b[0].value()
                    || a[0].value() == b[1].value()
                    || a[1].value() == b[0].value()
                    || a[1].value() == b[1].value();
            }
        }

        RangeSolver {
            tree,
            indexer,
            storage,
            iteration: 0,
            rank_cache,
            conflict_table,
        }
    }

    /// Run one CFR+ iteration using alternating traversals.
    ///
    /// First traversal updates IP regrets, second updates OOP regrets.
    pub fn run_iteration(&mut self) {
        self.iteration += 1;
        let nc = self.indexer.num_combos();
        let uniform = vec![1.0; nc];
        let t = self.iteration;

        // IP traversal: pass OOP reach, update IP regrets/strategy
        self.traverse_ip(0, &uniform, t);
        // OOP traversal: pass IP reach, update OOP regrets/strategy
        self.traverse_oop(0, &uniform, t);
    }

    /// IP traversal: returns `ev[ip_h]` from IP's perspective.
    ///
    /// Only passes opponent (OOP) reach. Updates IP regrets at IP decision nodes.
    fn traverse_ip(
        &mut self,
        node_id: NodeId,
        reach_oop: &[f64],
        t: u64,
    ) -> Vec<f64> {
        let nc = self.indexer.num_combos();
        let node = self.tree.get(node_id).expect("invalid node id");

        match node {
            Node::Terminal { .. } => {
                let folder = match node { Node::Terminal { folder, .. } => *folder, _ => unreachable!() };
                let pot = match node { Node::Terminal { pot, .. } => *pot, _ => unreachable!() };
                let board: Vec<Card> = node.board().to_vec();
                self.compute_terminal_ev_ip(&board, folder, pot, reach_oop)
            }

            Node::Decision { .. } => {
                let player = match node { Node::Decision { player, .. } => *player, _ => unreachable!() };
                let children: Vec<NodeId> = node.children().to_vec();
                let num_actions = children.len();

                let strategies: Vec<Vec<f64>> = (0..nc)
                    .map(|h| {
                        self.storage.node_data[node_id as usize].as_ref().unwrap().current_strategy(h)
                    })
                    .collect();

                let mut child_evs: Vec<Vec<f64>> = Vec::with_capacity(num_actions);

                match player {
                    Player::IP => {
                        // IP acts: recurse with same OOP reach for each action
                        for &child_id in &children {
                            let cev = self.traverse_ip(child_id, reach_oop, t);
                            child_evs.push(cev);
                        }
                        // node_ev = strategy-weighted sum (IP's strategy not in reach)
                        let mut node_ev = vec![0.0; nc];
                        for h in 0..nc {
                            for a in 0..num_actions {
                                node_ev[h] += strategies[h][a] * child_evs[a][h];
                            }
                        }
                        // Update IP regrets
                        for h in 0..nc {
                            let mut cf_values = vec![0.0; num_actions];
                            for a in 0..num_actions {
                                cf_values[a] = child_evs[a][h] - node_ev[h];
                            }
                            let data = self.storage.node_data[node_id as usize].as_mut().unwrap();
                            data.update_regrets(h, &cf_values);
                            data.accumulate_strategy(h, &strategies[h], t);
                        }
                        node_ev
                    }
                    Player::OOP => {
                        // OOP acts: split OOP reach by OOP's strategy, recurse
                        for (a, &child_id) in children.iter().enumerate() {
                            let mut new_reach_oop = reach_oop.to_vec();
                            for h in 0..nc { new_reach_oop[h] *= strategies[h][a]; }
                            let cev = self.traverse_ip(child_id, &new_reach_oop, t);
                            child_evs.push(cev);
                        }
                        // node_ev = sum (OOP's strategy already in reach)
                        let mut node_ev = vec![0.0; nc];
                        for h in 0..nc {
                            for a in 0..num_actions {
                                node_ev[h] += child_evs[a][h];
                            }
                        }
                        node_ev
                    }
                }
            }

            Node::Chance { .. } => {
                let children: Vec<NodeId> = node.children().to_vec();
                let mut ev_sum = vec![0.0; nc];
                let mut valid_count = vec![0.0_f64; nc];

                for &child_id in &children {
                    let dealt = self.tree.get(child_id).unwrap().board().last().unwrap().value();
                    let mut new_reach = reach_oop.to_vec();
                    for h in 0..nc {
                        let c = &self.indexer.combos[h];
                        if c[0].value() == dealt || c[1].value() == dealt { new_reach[h] = 0.0; }
                    }
                    let cev = self.traverse_ip(child_id, &new_reach, t);
                    for h in 0..nc {
                        let c = &self.indexer.combos[h];
                        if c[0].value() != dealt && c[1].value() != dealt {
                            ev_sum[h] += cev[h]; valid_count[h] += 1.0;
                        }
                    }
                }
                for h in 0..nc { if valid_count[h] > 0.0 { ev_sum[h] /= valid_count[h]; } }
                ev_sum
            }
        }
    }

    /// OOP traversal: returns `ev[oop_h]` from OOP's perspective.
    ///
    /// Only passes opponent (IP) reach. Updates OOP regrets at OOP decision nodes.
    fn traverse_oop(
        &mut self,
        node_id: NodeId,
        reach_ip: &[f64],
        t: u64,
    ) -> Vec<f64> {
        let nc = self.indexer.num_combos();
        let node = self.tree.get(node_id).expect("invalid node id");

        match node {
            Node::Terminal { .. } => {
                let folder = match node { Node::Terminal { folder, .. } => *folder, _ => unreachable!() };
                let pot = match node { Node::Terminal { pot, .. } => *pot, _ => unreachable!() };
                let board: Vec<Card> = node.board().to_vec();
                self.compute_terminal_ev_oop_perspective(&board, folder, pot, reach_ip)
            }

            Node::Decision { .. } => {
                let player = match node { Node::Decision { player, .. } => *player, _ => unreachable!() };
                let children: Vec<NodeId> = node.children().to_vec();
                let num_actions = children.len();

                let strategies: Vec<Vec<f64>> = (0..nc)
                    .map(|h| {
                        self.storage.node_data[node_id as usize].as_ref().unwrap().current_strategy(h)
                    })
                    .collect();

                let mut child_evs: Vec<Vec<f64>> = Vec::with_capacity(num_actions);

                match player {
                    Player::OOP => {
                        // OOP acts: recurse with same IP reach for each action
                        for &child_id in &children {
                            let cev = self.traverse_oop(child_id, reach_ip, t);
                            child_evs.push(cev);
                        }
                        // node_ev = strategy-weighted sum (OOP's strategy not in reach)
                        let mut node_ev = vec![0.0; nc];
                        for h in 0..nc {
                            for a in 0..num_actions {
                                node_ev[h] += strategies[h][a] * child_evs[a][h];
                            }
                        }
                        // Update OOP regrets
                        for h in 0..nc {
                            let mut cf_values = vec![0.0; num_actions];
                            for a in 0..num_actions {
                                cf_values[a] = child_evs[a][h] - node_ev[h];
                            }
                            let data = self.storage.node_data[node_id as usize].as_mut().unwrap();
                            data.update_regrets(h, &cf_values);
                            data.accumulate_strategy(h, &strategies[h], t);
                        }
                        node_ev
                    }
                    Player::IP => {
                        // IP acts: split IP reach by IP's strategy, recurse
                        for (a, &child_id) in children.iter().enumerate() {
                            let mut new_reach_ip = reach_ip.to_vec();
                            for h in 0..nc { new_reach_ip[h] *= strategies[h][a]; }
                            let cev = self.traverse_oop(child_id, &new_reach_ip, t);
                            child_evs.push(cev);
                        }
                        // node_ev = sum (IP's strategy already in reach)
                        let mut node_ev = vec![0.0; nc];
                        for h in 0..nc {
                            for a in 0..num_actions {
                                node_ev[h] += child_evs[a][h];
                            }
                        }
                        node_ev
                    }
                }
            }

            Node::Chance { .. } => {
                let children: Vec<NodeId> = node.children().to_vec();
                let mut ev_sum = vec![0.0; nc];
                let mut valid_count = vec![0.0_f64; nc];

                for &child_id in &children {
                    let dealt = self.tree.get(child_id).unwrap().board().last().unwrap().value();
                    let mut new_reach = reach_ip.to_vec();
                    for h in 0..nc {
                        let c = &self.indexer.combos[h];
                        if c[0].value() == dealt || c[1].value() == dealt { new_reach[h] = 0.0; }
                    }
                    let cev = self.traverse_oop(child_id, &new_reach, t);
                    for h in 0..nc {
                        let c = &self.indexer.combos[h];
                        if c[0].value() != dealt && c[1].value() != dealt {
                            ev_sum[h] += cev[h]; valid_count[h] += 1.0;
                        }
                    }
                }
                for h in 0..nc { if valid_count[h] > 0.0 { ev_sum[h] /= valid_count[h]; } }
                ev_sum
            }
        }
    }

    /// Compute terminal EV from IP's perspective.
    ///
    /// Returns `ev[ip_combo_idx]` = Σ_j(reach_oop[j] * payoff_IP(ip_combo, oop_combo_j))
    fn compute_terminal_ev_ip(
        &self,
        board: &[Card],
        folder: Option<Player>,
        pot: f64,
        reach_oop: &[f64],
    ) -> Vec<f64> {
        let nc = self.indexer.num_combos();
        let mut ev = vec![0.0; nc];

        match folder {
            Some(Player::IP) => {
                // IP folds → IP gets 0.0 per combo
                // ev[i] = 0.0 * Σ_j(reach_oop[j] for non-conflicting j) = 0.0
                // (already zero)
            }
            Some(Player::OOP) => {
                // OOP folds → IP wins pot
                // ev[i] = pot * Σ_j(reach_oop[j] for non-conflicting j)
                for i in 0..nc {
                    let mut opp_reach_sum = 0.0;
                    for j in 0..nc {
                        if !self.conflict_table[i * nc + j] {
                            opp_reach_sum += reach_oop[j];
                        }
                    }
                    ev[i] = pot * opp_reach_sum;
                }
            }
            None => {
                // Showdown — use precomputed ranks
                if let Some(ranks) = self.rank_cache.get(board) {
                    for i in 0..nc {
                        if ranks[i] == u16::MAX {
                            continue; // combo blocked by board
                        }
                        let ip_rank = ranks[i];
                        for j in 0..nc {
                            if self.conflict_table[i * nc + j] {
                                continue;
                            }
                            if ranks[j] == u16::MAX {
                                continue;
                            }
                            let oop_rank = ranks[j];
                            let equity = match ip_rank.cmp(&oop_rank) {
                                std::cmp::Ordering::Less => 1.0,    // IP wins
                                std::cmp::Ordering::Greater => 0.0, // OOP wins
                                std::cmp::Ordering::Equal => 0.5,   // tie
                            };
                            ev[i] += reach_oop[j] * pot * equity;
                        }
                    }
                } else {
                    // No rank cache for this board (incomplete board) → assume 50/50
                    for i in 0..nc {
                        let mut opp_reach_sum = 0.0;
                        for j in 0..nc {
                            if !self.conflict_table[i * nc + j] {
                                opp_reach_sum += reach_oop[j];
                            }
                        }
                        ev[i] = pot * 0.5 * opp_reach_sum;
                    }
                }
            }
        }

        ev
    }

    /// Compute terminal EV from OOP's perspective (indexed by OOP combo).
    ///
    /// Returns `ev[oop_combo_idx]` = Σ_i(reach_ip[i] * payoff_OOP(ip_combo_i, oop_combo))
    /// where payoff_OOP = pot - payoff_IP for zero-sum (both invested equally).
    fn compute_terminal_ev_oop_perspective(
        &self,
        board: &[Card],
        folder: Option<Player>,
        pot: f64,
        reach_ip: &[f64],
    ) -> Vec<f64> {
        let nc = self.indexer.num_combos();
        let mut ev = vec![0.0; nc];

        match folder {
            Some(Player::IP) => {
                // IP folds → OOP wins pot
                // ev_oop[j] = pot * Σ_i(reach_ip[i]) for non-conflicting i
                for j in 0..nc {
                    let mut ip_reach_sum = 0.0;
                    for i in 0..nc {
                        if !self.conflict_table[i * nc + j] {
                            ip_reach_sum += reach_ip[i];
                        }
                    }
                    ev[j] = pot * ip_reach_sum;
                }
            }
            Some(Player::OOP) => {
                // OOP folds → OOP gets 0
                // (already zero)
            }
            None => {
                // Showdown — use precomputed ranks
                if let Some(ranks) = self.rank_cache.get(board) {
                    for j in 0..nc {
                        if ranks[j] == u16::MAX {
                            continue;
                        }
                        let oop_rank = ranks[j];
                        for i in 0..nc {
                            if self.conflict_table[i * nc + j] {
                                continue;
                            }
                            if ranks[i] == u16::MAX {
                                continue;
                            }
                            let ip_rank = ranks[i];
                            // OOP equity = 1 - IP equity
                            let oop_equity = match ip_rank.cmp(&oop_rank) {
                                std::cmp::Ordering::Less => 0.0,    // IP wins → OOP gets 0
                                std::cmp::Ordering::Greater => 1.0, // OOP wins → OOP gets pot
                                std::cmp::Ordering::Equal => 0.5,   // tie
                            };
                            ev[j] += reach_ip[i] * pot * oop_equity;
                        }
                    }
                } else {
                    for j in 0..nc {
                        let mut ip_reach_sum = 0.0;
                        for i in 0..nc {
                            if !self.conflict_table[i * nc + j] {
                                ip_reach_sum += reach_ip[i];
                            }
                        }
                        ev[j] = pot * 0.5 * ip_reach_sum;
                    }
                }
            }
        }

        ev
    }

    /// Compute terminal EV indexed by OOP combo, but from IP's payoff perspective.
    ///
    /// Returns `ev[oop_combo_idx]` = Σ_i(reach_ip[i] * payoff_IP(ip_combo_i, oop_combo))
    /// Used by the OOP best-response traversal for exploitability calculation.
    fn compute_terminal_ev_oop_ip_payoff(
        &self,
        board: &[Card],
        folder: Option<Player>,
        pot: f64,
        reach_ip: &[f64],
    ) -> Vec<f64> {
        let nc = self.indexer.num_combos();
        let mut ev = vec![0.0; nc];

        match folder {
            Some(Player::IP) => {
                // IP folds → payoff_IP = 0 → ev[j] = 0
            }
            Some(Player::OOP) => {
                // OOP folds → payoff_IP = pot
                for j in 0..nc {
                    let mut ip_reach_sum = 0.0;
                    for i in 0..nc {
                        if !self.conflict_table[i * nc + j] {
                            ip_reach_sum += reach_ip[i];
                        }
                    }
                    ev[j] = pot * ip_reach_sum;
                }
            }
            None => {
                if let Some(ranks) = self.rank_cache.get(board) {
                    for j in 0..nc {
                        if ranks[j] == u16::MAX { continue; }
                        let oop_rank = ranks[j];
                        for i in 0..nc {
                            if self.conflict_table[i * nc + j] || ranks[i] == u16::MAX { continue; }
                            let ip_rank = ranks[i];
                            let ip_equity = match ip_rank.cmp(&oop_rank) {
                                std::cmp::Ordering::Less => 1.0,
                                std::cmp::Ordering::Greater => 0.0,
                                std::cmp::Ordering::Equal => 0.5,
                            };
                            ev[j] += reach_ip[i] * pot * ip_equity;
                        }
                    }
                } else {
                    for j in 0..nc {
                        let mut ip_reach_sum = 0.0;
                        for i in 0..nc {
                            if !self.conflict_table[i * nc + j] { ip_reach_sum += reach_ip[i]; }
                        }
                        ev[j] = pot * 0.5 * ip_reach_sum;
                    }
                }
            }
        }

        ev
    }

    /// Compute exploitability of current average strategies.
    ///
    /// Returns `(total_exploitability, ip_br_gain, oop_br_gain)`.
    ///
    /// exploit = (Σ ip_br_ev - Σ oop_br_ev) / num_valid_pairs
    /// where ip_br_ev[i] = Σ_j(payoff_IP when IP plays BR), indexed by IP combo
    /// and   oop_br_ev[j] = Σ_i(payoff_IP when OOP plays BR), indexed by OOP combo
    pub fn compute_exploitability(&self) -> (f64, f64, f64) {
        let nc = self.indexer.num_combos();
        let uniform = vec![1.0; nc];

        // IP BR: IP maximizes IP payoff, OOP plays avg.
        // Returns ev[ip_combo] = Σ_oop(reach_oop * payoff_IP under IP_BR, OOP_avg)
        let ip_br_ev = self.br_ip_traverse(0, &uniform);

        // OOP BR: OOP minimizes IP payoff, IP plays avg.
        // Returns ev[oop_combo] = Σ_ip(reach_ip * payoff_IP under IP_avg, OOP_BR)
        let oop_br_ev = self.br_oop_traverse(0, &uniform);

        // Count valid pairs for normalization
        let mut num_pairs = 0.0_f64;
        for i in 0..nc {
            for j in 0..nc {
                if !self.conflict_table[i * nc + j] {
                    num_pairs += 1.0;
                }
            }
        }

        let ip_br_total: f64 = ip_br_ev.iter().sum();
        let oop_br_total: f64 = oop_br_ev.iter().sum();

        if num_pairs > 0.0 {
            // exploit = (IP BR value - OOP BR value) / num_pairs
            // ip_br_gain = (IP BR - game value) / num_pairs
            // oop_br_gain = (game value - OOP BR) / num_pairs
            // We don't need avg separately: exploit = ip_br_gain + oop_br_gain
            let exploit = (ip_br_total - oop_br_total) / num_pairs;
            // Split roughly: attribute half to each player as approximation
            // (exact split requires avg_ev computation)
            let ip_br_gain = (exploit / 2.0).max(0.0);
            let oop_br_gain = (exploit / 2.0).max(0.0);
            (exploit.max(0.0), ip_br_gain, oop_br_gain)
        } else {
            (0.0, 0.0, 0.0)
        }
    }

    /// IP best-response traversal.
    ///
    /// Returns `ev[ip_combo]` = Σ_oop(reach_oop * payoff_IP) where IP plays BR
    /// and OOP plays average strategy.
    fn br_ip_traverse(&self, node_id: NodeId, reach_oop: &[f64]) -> Vec<f64> {
        let nc = self.indexer.num_combos();
        let node = self.tree.get(node_id).expect("invalid node id");

        match node {
            Node::Terminal { folder, pot, board, .. } => {
                let folder = *folder;
                let pot = *pot;
                let board = board.clone();
                self.compute_terminal_ev_ip(&board, folder, pot, reach_oop)
            }

            Node::Decision { player, children, .. } => {
                let acting = *player;
                let children: Vec<NodeId> = children.clone();
                let num_actions = children.len();

                if acting == Player::IP {
                    // IP plays BR: for each IP combo, pick action maximizing IP's EV
                    let mut child_evs: Vec<Vec<f64>> = Vec::with_capacity(num_actions);
                    for &child_id in &children {
                        let cev = self.br_ip_traverse(child_id, reach_oop);
                        child_evs.push(cev);
                    }
                    let mut ev = vec![0.0; nc];
                    for h in 0..nc {
                        ev[h] = f64::NEG_INFINITY;
                        for a in 0..num_actions {
                            if child_evs[a][h] > ev[h] {
                                ev[h] = child_evs[a][h];
                            }
                        }
                        if ev[h] == f64::NEG_INFINITY {
                            ev[h] = 0.0;
                        }
                    }
                    ev
                } else {
                    // OOP plays avg strategy, modifying OOP reach
                    let strategies: Vec<Vec<f64>> = (0..nc)
                        .map(|h| {
                            self.storage.node_data[node_id as usize]
                                .as_ref()
                                .unwrap()
                                .average_strategy(h)
                        })
                        .collect();
                    let mut child_evs: Vec<Vec<f64>> = Vec::with_capacity(num_actions);
                    for (a, &child_id) in children.iter().enumerate() {
                        let mut new_reach_oop = reach_oop.to_vec();
                        for h in 0..nc {
                            new_reach_oop[h] *= strategies[h][a];
                        }
                        let cev = self.br_ip_traverse(child_id, &new_reach_oop);
                        child_evs.push(cev);
                    }
                    // Sum across actions (reach already applied)
                    let mut ev = vec![0.0; nc];
                    for h in 0..nc {
                        for a in 0..num_actions {
                            ev[h] += child_evs[a][h];
                        }
                    }
                    ev
                }
            }

            Node::Chance { children, .. } => {
                let children: Vec<NodeId> = children.clone();
                let mut ev_sum = vec![0.0; nc];
                let mut valid_count = vec![0.0_f64; nc];
                for &child_id in &children {
                    let dealt = self.tree.get(child_id).unwrap().board().last().unwrap().value();
                    let mut new_reach = reach_oop.to_vec();
                    for h in 0..nc {
                        let c = &self.indexer.combos[h];
                        if c[0].value() == dealt || c[1].value() == dealt { new_reach[h] = 0.0; }
                    }
                    let cev = self.br_ip_traverse(child_id, &new_reach);
                    for h in 0..nc {
                        let c = &self.indexer.combos[h];
                        if c[0].value() != dealt && c[1].value() != dealt {
                            ev_sum[h] += cev[h]; valid_count[h] += 1.0;
                        }
                    }
                }
                for h in 0..nc { if valid_count[h] > 0.0 { ev_sum[h] /= valid_count[h]; } }
                ev_sum
            }
        }
    }

    /// OOP best-response traversal.
    ///
    /// Returns `ev[oop_combo]` = Σ_ip(reach_ip * payoff_IP) where OOP plays BR
    /// (minimizing IP's payoff) and IP plays average strategy.
    fn br_oop_traverse(&self, node_id: NodeId, reach_ip: &[f64]) -> Vec<f64> {
        let nc = self.indexer.num_combos();
        let node = self.tree.get(node_id).expect("invalid node id");

        match node {
            Node::Terminal { folder, pot, board, .. } => {
                let folder = *folder;
                let pot = *pot;
                let board = board.clone();
                // Terminal EV: IP payoff indexed by OOP combo
                self.compute_terminal_ev_oop_ip_payoff(&board, folder, pot, reach_ip)
            }

            Node::Decision { player, children, .. } => {
                let acting = *player;
                let children: Vec<NodeId> = children.clone();
                let num_actions = children.len();

                if acting == Player::OOP {
                    // OOP plays BR: for each OOP combo, pick action minimizing IP's EV
                    let mut child_evs: Vec<Vec<f64>> = Vec::with_capacity(num_actions);
                    for &child_id in &children {
                        let cev = self.br_oop_traverse(child_id, reach_ip);
                        child_evs.push(cev);
                    }
                    // OOP picks MIN (minimizes IP payoff = maximizes OOP payoff)
                    let mut ev = vec![0.0; nc];
                    for h in 0..nc {
                        ev[h] = f64::INFINITY;
                        for a in 0..num_actions {
                            if child_evs[a][h] < ev[h] {
                                ev[h] = child_evs[a][h];
                            }
                        }
                        if ev[h] == f64::INFINITY {
                            ev[h] = 0.0;
                        }
                    }
                    ev
                } else {
                    // IP plays avg strategy, modifying IP reach
                    let strategies: Vec<Vec<f64>> = (0..nc)
                        .map(|h| {
                            self.storage.node_data[node_id as usize]
                                .as_ref()
                                .unwrap()
                                .average_strategy(h)
                        })
                        .collect();
                    let mut child_evs: Vec<Vec<f64>> = Vec::with_capacity(num_actions);
                    for (a, &child_id) in children.iter().enumerate() {
                        let mut new_reach_ip = reach_ip.to_vec();
                        for h in 0..nc {
                            new_reach_ip[h] *= strategies[h][a];
                        }
                        let cev = self.br_oop_traverse(child_id, &new_reach_ip);
                        child_evs.push(cev);
                    }
                    let mut ev = vec![0.0; nc];
                    for h in 0..nc {
                        for a in 0..num_actions {
                            ev[h] += child_evs[a][h];
                        }
                    }
                    ev
                }
            }

            Node::Chance { children, .. } => {
                let children: Vec<NodeId> = children.clone();
                let mut ev_sum = vec![0.0; nc];
                let mut valid_count = vec![0.0_f64; nc];
                for &child_id in &children {
                    let dealt = self.tree.get(child_id).unwrap().board().last().unwrap().value();
                    let mut new_reach = reach_ip.to_vec();
                    for h in 0..nc {
                        let c = &self.indexer.combos[h];
                        if c[0].value() == dealt || c[1].value() == dealt { new_reach[h] = 0.0; }
                    }
                    let cev = self.br_oop_traverse(child_id, &new_reach);
                    for h in 0..nc {
                        let c = &self.indexer.combos[h];
                        if c[0].value() != dealt && c[1].value() != dealt {
                            ev_sum[h] += cev[h]; valid_count[h] += 1.0;
                        }
                    }
                }
                for h in 0..nc { if valid_count[h] > 0.0 { ev_sum[h] /= valid_count[h]; } }
                ev_sum
            }
        }
    }

    /// Get the average strategy for a hand at a decision node.
    pub fn average_strategy(&self, node_id: NodeId, hand_idx: usize) -> Vec<f64> {
        self.storage.node_data[node_id as usize]
            .as_ref()
            .expect("not a decision node")
            .average_strategy(hand_idx)
    }

    /// Number of hand combos.
    pub fn num_hands(&self) -> usize {
        self.indexer.num_combos()
    }

    /// Get the hand combo at index.
    pub fn hand_at(&self, idx: usize) -> [Card; 2] {
        self.indexer.combos[idx]
    }

    /// Estimate memory usage.
    pub fn memory_usage(&self) -> MemoryReport {
        let tree_bytes = self.tree.nodes.len() * std::mem::size_of::<Node>() * 3; // rough estimate
        let storage_bytes = self.storage.memory_bytes();
        let rank_cache_bytes = self.rank_cache.memory_bytes();
        let conflict_table_bytes = self.conflict_table.len() * std::mem::size_of::<bool>();

        let total = tree_bytes + storage_bytes + rank_cache_bytes + conflict_table_bytes;
        MemoryReport {
            tree_bytes,
            storage_bytes,
            rank_cache_bytes,
            conflict_table_bytes,
            total_bytes: total,
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Action, Card, Node, Street};

    fn river_board() -> Vec<Card> {
        // As Kh 7d 2c 5s
        vec![
            Card::new(12),
            Card::new(24),
            Card::new(31),
            Card::new(39),
            Card::new(3),
        ]
    }

    fn small_river_tree() -> GameTree {
        let board = river_board();
        let mut nodes = Vec::new();

        // Node 0: OOP decision (check/bet)
        nodes.push(Node::Decision {
            id: 0,
            infoset_id: 0,
            player: Player::OOP,
            street: Street::River,
            parent: None,
            children: vec![1, 4],
            actions: vec![Action::Check, Action::Bet { size: 4.5 }],
            pot: 6.0,
            stacks: [97.0, 97.0],
            board: board.clone(),
            bet_sequence: vec![],
        });

        // Node 1: IP decision after check (check/bet)
        nodes.push(Node::Decision {
            id: 1,
            infoset_id: 1,
            player: Player::IP,
            street: Street::River,
            parent: Some(0),
            children: vec![2, 3],
            actions: vec![Action::Check, Action::Bet { size: 4.5 }],
            pot: 6.0,
            stacks: [97.0, 97.0],
            board: board.clone(),
            bet_sequence: vec![Action::Check],
        });

        // Node 2: Showdown (check-check)
        nodes.push(Node::Terminal {
            id: 2,
            parent: Some(1),
            folder: None,
            pot: 6.0,
            stacks: [97.0, 97.0],
            board: board.clone(),
            hole_cards: [None, None],
        });

        // Node 3: OOP faces bet (fold/call)
        nodes.push(Node::Decision {
            id: 3,
            infoset_id: 3,
            player: Player::OOP,
            street: Street::River,
            parent: Some(1),
            children: vec![6, 7],
            actions: vec![Action::Fold, Action::Call],
            pot: 10.5,
            stacks: [92.5, 97.0],
            board: board.clone(),
            bet_sequence: vec![Action::Check, Action::Bet { size: 4.5 }],
        });

        // Node 4: IP faces bet (fold/call)
        nodes.push(Node::Decision {
            id: 4,
            infoset_id: 4,
            player: Player::IP,
            street: Street::River,
            parent: Some(0),
            children: vec![5, 8],
            actions: vec![Action::Fold, Action::Call],
            pot: 10.5,
            stacks: [97.0, 92.5],
            board: board.clone(),
            bet_sequence: vec![Action::Bet { size: 4.5 }],
        });

        // Node 5: IP folds
        nodes.push(Node::Terminal {
            id: 5,
            parent: Some(4),
            folder: Some(Player::IP),
            pot: 10.5,
            stacks: [97.0, 92.5],
            board: board.clone(),
            hole_cards: [None, None],
        });

        // Node 6: OOP folds
        nodes.push(Node::Terminal {
            id: 6,
            parent: Some(3),
            folder: Some(Player::OOP),
            pot: 10.5,
            stacks: [92.5, 97.0],
            board: board.clone(),
            hole_cards: [None, None],
        });

        // Node 7: Showdown after check-bet-call
        nodes.push(Node::Terminal {
            id: 7,
            parent: Some(3),
            folder: None,
            pot: 15.0,
            stacks: [92.5, 92.5],
            board: board.clone(),
            hole_cards: [None, None],
        });

        // Node 8: Showdown after bet-call
        nodes.push(Node::Terminal {
            id: 8,
            parent: Some(4),
            folder: None,
            pot: 15.0,
            stacks: [92.5, 92.5],
            board: board.clone(),
            hole_cards: [None, None],
        });

        GameTree { nodes }
    }

    #[test]
    fn test_hand_indexer_counts() {
        let board = river_board();
        let indexer = HandIndexer::new(&board);
        // C(47, 2) = 1081 combos for a 5-card board
        assert_eq!(indexer.num_combos(), 1081);
    }

    #[test]
    fn test_rank_cache_matches_evaluator() {
        let board = river_board();
        let tree = small_river_tree();
        let indexer = HandIndexer::new(&board);
        let cache = RankCache::build(&tree, &indexer.combos);

        let eval = CactusKevEvaluator::new();
        let board5 = [
            Card::new(12),
            Card::new(24),
            Card::new(31),
            Card::new(39),
            Card::new(3),
        ];

        let ranks = cache.get(&board).unwrap();
        for (i, combo) in indexer.combos.iter().enumerate() {
            if ranks[i] == u16::MAX {
                // Should be blocked by board
                assert!(
                    board
                        .iter()
                        .any(|bc| bc.value() == combo[0].value() || bc.value() == combo[1].value()),
                    "combo {} marked blocked but doesn't conflict",
                    i
                );
            } else {
                let direct = eval.evaluate_7cards(board5, *combo).value();
                assert_eq!(
                    ranks[i], direct,
                    "rank mismatch at combo {}: cached={} direct={}",
                    i, ranks[i], direct
                );
            }
        }
    }

    #[test]
    fn test_conflict_table_symmetry() {
        let board = river_board();
        let tree = small_river_tree();
        let solver = RangeSolver::new(tree, &board);
        let nc = solver.indexer.num_combos();

        for i in 0..nc.min(100) {
            for j in 0..nc.min(100) {
                assert_eq!(
                    solver.conflict_table[i * nc + j],
                    solver.conflict_table[j * nc + i],
                    "conflict table not symmetric at ({}, {})",
                    i,
                    j
                );
            }
        }
    }

    #[test]
    fn test_range_solver_creates() {
        let tree = small_river_tree();
        let board = river_board();
        let solver = RangeSolver::new(tree, &board);
        assert_eq!(solver.iteration, 0);
        assert_eq!(solver.num_hands(), 1081);
    }

    #[test]
    fn test_range_solver_iteration_runs() {
        let tree = small_river_tree();
        let board = river_board();
        let mut solver = RangeSolver::new(tree, &board);
        solver.run_iteration();
        assert_eq!(solver.iteration, 1);
    }

    #[test]
    fn test_range_solver_exploitability_decreases() {
        let tree = small_river_tree();
        let board = river_board();
        let mut solver = RangeSolver::new(tree, &board);

        for _ in 0..50 {
            solver.run_iteration();
        }
        let (early, _, _) = solver.compute_exploitability();

        for _ in 0..450 {
            solver.run_iteration();
        }
        let (late, _, _) = solver.compute_exploitability();

        assert!(
            late <= early + 0.1,
            "exploitability should decrease: early={:.4} late={:.4}",
            early,
            late
        );
    }

    #[test]
    fn test_memory_report() {
        let tree = small_river_tree();
        let board = river_board();
        let solver = RangeSolver::new(tree, &board);
        let report = solver.memory_usage();
        assert!(report.total_bytes > 0);
        assert!(report.storage_bytes > 0);
    }
}
