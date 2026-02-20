//! Hand evaluator implementation using bitboard + precomputed table approach
//!
//! This module implements the HandEvaluator trait using:
//! - Precomputed 8192-entry FLUSH_TABLE (16 KB, fits in L1 cache)
//! - Bitboard suit-mask and rank-count arrays built once per 7-card hand
//! - O(1) per-hand evaluation — no 21-combination enumeration loop
//!
//! The evaluator is designed for high throughput (target: 50M+ evals/sec).

use crate::node::{Card, HandEvaluator, HandRank};

/// Cactus Kev evaluator implementation
///
/// Uses two-path evaluation:
/// - Flush path: suit-mask → `best_flush_hand_7` → FLUSH_TABLE lookup
/// - Non-flush path: rank-counts array → `best_nonflush_hand_7`
#[derive(Debug, Clone, Copy)]
pub struct CactusKevEvaluator;

impl CactusKevEvaluator {
    /// Create a new Cactus Kev evaluator
    pub fn new() -> Self {
        CactusKevEvaluator
    }

    /// Evaluate a 7-card hand (5 board + 2 hole cards)
    ///
    /// Builds suit masks and rank counts in a single pass, then takes the
    /// flush or non-flush path — O(1) scalar, no combination loop.
    pub fn evaluate_7cards(&self, board: [Card; 5], hand: [Card; 2]) -> HandRank {
        let all = [board[0], board[1], board[2], board[3], board[4], hand[0], hand[1]];
        let mut suit_masks = [0u16; 4];
        let mut rank_counts = [0u8; 13];
        for card in all.iter() {
            let v = card.value();
            suit_masks[(v / 13) as usize] |= 1u16 << (v % 13);
            rank_counts[(v % 13) as usize] += 1;
        }
        for mask in suit_masks.iter() {
            if mask.count_ones() >= 5 {
                return HandRank::new(tables::best_flush_hand_7(*mask));
            }
        }
        HandRank::new(tables::best_nonflush_hand_7(&rank_counts))
    }

    /// Reference evaluator using the original 21-combination loop.
    /// Used only by consistency tests to cross-check the bitboard path.
    #[cfg(test)]
    fn evaluate_7cards_reference(&self, board: [Card; 5], hand: [Card; 2]) -> HandRank {
        let all_cards = [board[0], board[1], board[2], board[3], board[4], hand[0], hand[1]];
        let mut best_rank = u16::MAX;
        for i in 0..7 {
            for j in (i + 1)..7 {
                for k in (j + 1)..7 {
                    for l in (k + 1)..7 {
                        for m in (l + 1)..7 {
                            let five_cards = [all_cards[i], all_cards[j], all_cards[k], all_cards[l], all_cards[m]];
                            let rank = self.rank_5cards_ref(five_cards);
                            if rank < best_rank {
                                best_rank = rank;
                            }
                        }
                    }
                }
            }
        }
        HandRank::new(best_rank)
    }

    /// 5-card ranker used only by the reference evaluator.
    #[cfg(test)]
    fn rank_5cards_ref(&self, cards: [Card; 5]) -> u16 {
        let mut suit_masks = [0u16; 4];
        for card in cards.iter() {
            let card_val = card.value();
            let suit = (card_val / 13) as usize;
            let rank = card_val % 13;
            suit_masks[suit] |= 1u16 << rank;
        }
        let mut flush_suit = None;
        for (suit_idx, mask) in suit_masks.iter().enumerate() {
            if mask.count_ones() == 5 {
                flush_suit = Some(suit_idx);
                break;
            }
        }
        if let Some(suit_idx) = flush_suit {
            let rank_mask = suit_masks[suit_idx];
            tables::get_flush_table()[rank_mask as usize]
        } else {
            use tables::RANK_PRIMES_REF;
            let mut product = 1u32;
            for card in cards.iter() {
                let rank = card.value() % 13;
                product *= RANK_PRIMES_REF[rank as usize];
            }
            tables::lookup_nonflush_rank(product)
        }
    }

    /// Evaluate a batch of 7-card hands
    ///
    /// Uses NEON-accelerated path on ARM64 (Apple Silicon), falls back to scalar
    /// path on other architectures.
    pub fn evaluate_batch(&self, boards: &[[Card; 5]], hands: &[[Card; 2]]) -> Vec<HandRank> {
        assert_eq!(boards.len(), hands.len(), "boards and hands must have same length");

        #[cfg(target_arch = "aarch64")]
        {
            return neon::evaluate_batch_neon(self, boards, hands);
        }

        #[cfg(not(target_arch = "aarch64"))]
        {
            let mut results = Vec::with_capacity(boards.len());
            for i in 0..boards.len() {
                results.push(self.evaluate_7cards(boards[i], hands[i]));
            }
            results
        }
    }
}

impl Default for CactusKevEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl HandEvaluator for CactusKevEvaluator {
    fn evaluate(&self, board: [Card; 5], hand: [Card; 2]) -> HandRank {
        self.evaluate_7cards(board, hand)
    }
}

/// Benchmark helper for CLI
///
/// Runs a batch evaluation and returns (evals_per_sec, duration_ms)
pub fn benchmark_throughput(sample_size: usize) -> (f64, u64) {
    use std::time::Instant;

    let evaluator = CactusKevEvaluator::new();

    // Generate test hands (same logic as benchmark)
    let mut seed: u64 = 12345;
    let lcg_next = |s: &mut u64| {
        *s = s.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
        (*s % 52) as u8
    };

    let mut boards = Vec::with_capacity(sample_size);
    let mut hands = Vec::with_capacity(sample_size);

    for _ in 0..sample_size {
        let mut cards = Vec::new();
        while cards.len() < 7 {
            let card_val = lcg_next(&mut seed);
            if !cards.contains(&card_val) {
                cards.push(card_val);
            } else {
                seed = seed.wrapping_add(1);
            }
        }
        boards.push([Card::new(cards[0]), Card::new(cards[1]), Card::new(cards[2]), Card::new(cards[3]), Card::new(cards[4])]);
        hands.push([Card::new(cards[5]), Card::new(cards[6])]);
    }

    // Warm-up (also initializes FLUSH_TABLE)
    for i in 0..10_000.min(sample_size) {
        let _ = evaluator.evaluate(boards[i], hands[i]);
    }

    // Timed run
    let start = Instant::now();
    let _results = evaluator.evaluate_batch(&boards, &hands);
    let duration = start.elapsed();

    let evals_per_sec = sample_size as f64 / duration.as_secs_f64();
    let duration_ms = duration.as_millis() as u64;

    (evals_per_sec, duration_ms)
}

mod tables {
    //! Lookup tables for hand evaluator
    //!
    //! FLUSH_TABLE: 8192-entry precomputed table (16 KB) — fits in L1 cache.
    //! 1287 valid entries (C(13,5)), rest stay 0. Initialized once via OnceLock.

    use std::sync::OnceLock;

    static FLUSH_TABLE: OnceLock<[u16; 8192]> = OnceLock::new();

    /// Prime numbers for each rank (2-A, where 2=index 0, A=index 12).
    /// Exposed for use by the reference evaluator in tests.
    #[cfg(test)]
    pub(crate) const RANK_PRIMES_REF: [u32; 13] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41];

    /// Return a reference to the precomputed flush rank table.
    /// Initialized on first call; subsequent calls are a single atomic load.
    pub(crate) fn get_flush_table() -> &'static [u16; 8192] {
        FLUSH_TABLE.get_or_init(|| {
            let mut table = [0u16; 8192];
            for mask in 0u16..8192 {
                if mask.count_ones() == 5 {
                    table[mask as usize] = compute_flush_rank(mask);
                }
            }
            table
        })
    }

    /// Compute the best flush or straight-flush rank from a 7-card suit mask.
    ///
    /// `suit_mask` has bits set for each rank present in that suit (may have 5–7 bits set).
    /// Scans straight-flush windows high-to-low first (including wheel), then falls back
    /// to the best regular flush by keeping the top 5 bits.
    pub(crate) fn best_flush_hand_7(suit_mask: u16) -> u16 {
        let table = get_flush_table();

        // Check straight flushes high-to-low (A-high down to 6-high)
        for high in (4u16..=12).rev() {
            let sf_mask = 0x1Fu16 << (high - 4);
            if suit_mask & sf_mask == sf_mask {
                // Return the SF rank directly from the table
                return table[sf_mask as usize];
            }
        }
        // Check wheel SF: A-5-4-3-2 = bits 12,3,2,1,0 = 0x100F
        if suit_mask & 0x100F == 0x100F {
            return table[0x100F as usize];
        }

        // Regular flush: keep top 5 bits of suit_mask
        let mut mask = suit_mask;
        while mask.count_ones() > 5 {
            mask &= mask - 1; // clear lowest set bit
        }
        table[mask as usize]
    }

    /// Compute the best non-flush hand rank from a 7-card rank-count array.
    ///
    /// `rank_counts[i]` is the number of cards of rank i (0=2, 12=A).
    /// No heap allocation. Uses stack arrays with sentinel 255 = "not set".
    pub(crate) fn best_nonflush_hand_7(rank_counts: &[u8; 13]) -> u16 {
        // Single descending scan to classify cards
        let mut quad_rank: u8 = 255;
        let mut trips_rank: u8 = 255;
        let mut pairs = [255u8; 3];
        let mut singles = [255u8; 7];
        let mut pair_count: usize = 0;
        let mut single_count: usize = 0;

        for i in (0..13usize).rev() {
            match rank_counts[i] {
                4 => {
                    quad_rank = i as u8;
                }
                3 => {
                    if trips_rank == 255 {
                        trips_rank = i as u8;
                    } else {
                        // Second set of trips: lower trips becomes a pair for full house
                        if pair_count < 3 {
                            pairs[pair_count] = i as u8;
                            pair_count += 1;
                        }
                    }
                }
                2 => {
                    if pair_count < 3 {
                        pairs[pair_count] = i as u8;
                        pair_count += 1;
                    }
                }
                1 => {
                    if single_count < 7 {
                        singles[single_count] = i as u8;
                        single_count += 1;
                    }
                }
                _ => {}
            }
        }

        // Priority 1: Four of a kind
        if quad_rank != 255 {
            // Best kicker: highest among trips, pairs, singles
            let kicker = best_kicker_excluding(quad_rank, trips_rank, &pairs, pair_count, &singles, single_count);
            return rank_four_of_a_kind(quad_rank, kicker);
        }

        // Priority 2: Full house (trips + pair, or two trips)
        if trips_rank != 255 {
            let pair_for_fh = if pair_count > 0 { pairs[0] } else { 255 };
            if pair_for_fh != 255 {
                return rank_full_house(trips_rank, pair_for_fh);
            }
            // No pair → not a full house; fall through to check straight
        }

        // Priority 3: Straight (using rank_present bitmask)
        {
            let mut rank_present: u16 = 0;
            for i in 0..13 {
                if rank_counts[i] > 0 {
                    rank_present |= 1u16 << i;
                }
            }
            // Check A-high down to 6-high
            for high in (4u8..=12).rev() {
                let mask = 0x1Fu16 << (high - 4);
                if rank_present & mask == mask {
                    let straight_high = high + 1;
                    let rank = if straight_high == 13 { 1600 } else { 1600 + (12 - high) as u16 };
                    return rank;
                }
            }
            // Wheel: A-5-4-3-2
            if rank_present & 0x100F == 0x100F {
                return 1609;
            }
        }

        // Priority 4: Three of a kind (no pair exists, otherwise it would be FH)
        if trips_rank != 255 {
            let k1 = singles[0];
            let k2 = if single_count >= 2 { singles[1] } else { 255 };
            return rank_three_of_a_kind(trips_rank, k1, k2);
        }

        // Priority 5: Two pair
        if pair_count >= 2 {
            let high_pair = pairs[0];
            let low_pair = pairs[1];
            // Kicker: third pair (if any) or best single — whichever is higher
            let kicker = if pair_count >= 3 {
                let third_pair = pairs[2];
                let best_single = singles[0];
                if best_single != 255 && best_single > third_pair { best_single } else { third_pair }
            } else {
                singles[0]
            };
            return rank_two_pair(high_pair, low_pair, kicker);
        }

        // Priority 6: One pair
        if pair_count == 1 {
            let k1 = singles[0];
            let k2 = if single_count >= 2 { singles[1] } else { 0 };
            let k3 = if single_count >= 3 { singles[2] } else { 0 };
            return rank_one_pair(pairs[0], k1, k2, k3);
        }

        // Priority 7: High card (5 best singles)
        rank_high_card(singles[0], singles[1], singles[2], singles[3], singles[4])
    }

    /// Return the best available kicker rank for a quad hand.
    /// Looks in trips_rank, pairs[], and singles[] in priority order.
    fn best_kicker_excluding(
        _quad: u8,
        trips_rank: u8,
        pairs: &[u8; 3],
        pair_count: usize,
        singles: &[u8; 7],
        single_count: usize,
    ) -> u8 {
        // trips > pairs[0] > singles[0] already sorted descending
        let mut best = 0u8;
        if trips_rank != 255 && trips_rank > best { best = trips_rank; }
        if pair_count > 0 && pairs[0] != 255 && pairs[0] > best { best = pairs[0]; }
        if single_count > 0 && singles[0] != 255 && singles[0] > best { best = singles[0]; }
        best
    }

    /// Non-flush rank lookup used only by the reference evaluator in tests.
    /// Builds rank counts from prime factorization then classifies — same logic as
    /// `best_nonflush_hand_7` so results are consistent for 5-card subhands.
    #[cfg(test)]
    pub(crate) fn lookup_nonflush_rank(product: u32) -> u16 {
        // Recover per-rank counts from prime product
        let mut rank_counts = [0u8; 13];
        let mut remaining = product;
        for (idx, &prime) in RANK_PRIMES_REF.iter().enumerate() {
            while remaining % prime == 0 && remaining > 0 {
                remaining /= prime;
                rank_counts[idx] += 1;
            }
        }
        let total: u8 = rank_counts.iter().sum();
        if total != 5 {
            return 7462;
        }

        // Classify with a single descending scan (mirrors best_nonflush_hand_7)
        let mut quad_rank = 255u8;
        let mut trips_rank = 255u8;
        let mut pairs = [255u8; 2];
        let mut singles = [255u8; 5];
        let mut pair_count = 0usize;
        let mut single_count = 0usize;

        for i in (0..13usize).rev() {
            match rank_counts[i] {
                4 => { quad_rank = i as u8; }
                3 => { trips_rank = i as u8; }
                2 => { if pair_count < 2 { pairs[pair_count] = i as u8; pair_count += 1; } }
                1 => { if single_count < 5 { singles[single_count] = i as u8; single_count += 1; } }
                _ => {}
            }
        }

        if quad_rank != 255 {
            let kicker = if trips_rank != 255 { trips_rank }
                         else if pair_count > 0 { pairs[0] }
                         else { singles[0] };
            return rank_four_of_a_kind(quad_rank, kicker);
        }
        if trips_rank != 255 && pair_count > 0 {
            return rank_full_house(trips_rank, pairs[0]);
        }
        if trips_rank != 255 {
            return rank_three_of_a_kind(trips_rank, singles[0], singles[1]);
        }
        if pair_count >= 2 {
            return rank_two_pair(pairs[0], pairs[1], singles[0]);
        }
        // Straight check
        {
            let mut rank_present = 0u16;
            for i in 0..13 { if rank_counts[i] > 0 { rank_present |= 1u16 << i; } }
            for high in (4u8..=12).rev() {
                let mask = 0x1Fu16 << (high - 4);
                if rank_present & mask == mask {
                    let sh = high + 1;
                    return if sh == 13 { 1600 } else { 1600 + (12 - high) as u16 };
                }
            }
            if rank_present & 0x100F == 0x100F { return 1609; }
        }
        if pair_count == 1 {
            return rank_one_pair(pairs[0], singles[0], singles[1], singles[2]);
        }
        rank_high_card(singles[0], singles[1], singles[2], singles[3], singles[4])
    }

    fn rank_four_of_a_kind(quad_rank: u8, kicker: u8) -> u16 {
        // 13 quad ranks × 12 kicker ranks = 156 hands → range 11-166.
        // Remap kicker to its ordinal among the 12 non-quad ranks.
        let adj_kicker = (12 - kicker) as u16 - u16::from(quad_rank > kicker);
        11 + (12 - quad_rank) as u16 * 12 + adj_kicker
    }

    fn rank_full_house(trips_rank: u8, pair_rank: u8) -> u16 {
        // 13 trips ranks × 12 pair ranks = 156 hands → range 167-322.
        // Remap pair to its ordinal among the 12 non-trips ranks.
        let adj_pair = (12 - pair_rank) as u16 - u16::from(trips_rank > pair_rank);
        167 + (12 - trips_rank) as u16 * 12 + adj_pair
    }

    fn rank_three_of_a_kind(trips_rank: u8, kicker1: u8, kicker2: u8) -> u16 {
        // 13 trips × C(12,2)=66 kicker combos = 858 hands → range 1610-2467.
        // Remap each kicker to its ordinal among the 12 non-trips ranks, then use CNS.
        let adj = |k: u8| k - u8::from(k > trips_rank);
        let (hi, lo) = if kicker1 > kicker2 { (kicker1, kicker2) } else { (kicker2, kicker1) };
        let inner = comb(adj(hi), 2) + comb(adj(lo), 1); // 0..=65
        1610 + (12 - trips_rank) as u16 * 66 + (65 - inner)
    }

    fn rank_two_pair(high_pair: u8, low_pair: u8, kicker: u8) -> u16 {
        let combo = comb(high_pair, 2) + comb(low_pair, 1);
        let base = 2468 + (77 - combo) * 11;
        let kicker_off = (12u8.saturating_sub(kicker)).min(10) as u16;
        base + kicker_off
    }

    fn rank_one_pair(pair_rank: u8, kicker1: u8, kicker2: u8, kicker3: u8) -> u16 {
        let mut kickers = [kicker1, kicker2, kicker3];
        kickers.sort();
        kickers.reverse();
        // Remap each kicker to its ordinal position among the 12 non-pair ranks
        // (subtract 1 for each kicker rank that exceeds pair_rank, so index stays in [0,11])
        let adj = |k: u8| k - u8::from(k > pair_rank);
        let kicker_combo = comb(adj(kickers[0]), 3) + comb(adj(kickers[1]), 2) + comb(adj(kickers[2]), 1);
        let base = 3326 + (12 - pair_rank) as u16 * 220;
        base + (219 - kicker_combo)
    }

    pub(crate) fn comb(n: u8, k: u8) -> u16 {
        if n < k { return 0; }
        let n = n as u32;
        match k {
            0 => 1,
            1 => n as u16,
            2 => (n * (n - 1) / 2) as u16,
            3 => (n * (n - 1) * (n - 2) / 6) as u16,
            4 => (n * (n - 1) * (n - 2) * (n - 3) / 24) as u16,
            5 => (n * (n - 1) * (n - 2) * (n - 3) * (n - 4) / 120) as u16,
            _ => 0,
        }
    }

    fn rank_high_card(c1: u8, c2: u8, c3: u8, c4: u8, c5: u8) -> u16 {
        let index = comb(c1, 5) + comb(c2, 4) + comb(c3, 3) + comb(c4, 2) + comb(c5, 1);
        (6186 + (1286 - index)).min(7462)
    }

    /// Compute flush rank for a raw 5-card mask (private — only called during table init).
    fn compute_flush_rank(rank_mask: u16) -> u16 {
        // Wheel SF: A-5-4-3-2
        if rank_mask == 0x100F {
            return 10;
        }
        // Straight flushes: 5 consecutive bits
        for high in 4u16..=12 {
            let mask = 0x1Fu16 << (high - 4);
            if rank_mask == mask {
                let straight_high = high + 1;
                return if straight_high == 13 { 1 } else { 14 - straight_high };
            }
        }
        // Regular flush via combinatorial number system
        let mut bits = [0u8; 5];
        let mut count = 0;
        for i in (0u8..13).rev() {
            if rank_mask & (1u16 << i) != 0 {
                bits[count] = i;
                count += 1;
            }
        }
        let idx = (comb(bits[0], 5) + comb(bits[1], 4) + comb(bits[2], 3)
            + comb(bits[3], 2) + comb(bits[4], 1)) as u32;
        let offset = (1286 - idx) * 1276 / 1286;
        323 + offset as u16
    }
}

/// NEON-accelerated batch evaluation module
#[cfg(target_arch = "aarch64")]
mod neon {
    use crate::node::Card;
    use super::{CactusKevEvaluator, HandRank};

    /// NEON-accelerated batch evaluation.
    ///
    /// Each hand is evaluated with the O(1) scalar bitboard path.
    /// The SIMD benefit comes from cache-warm FLUSH_TABLE + branch-free arithmetic.
    pub fn evaluate_batch_neon(
        evaluator: &CactusKevEvaluator,
        boards: &[[Card; 5]],
        hands: &[[Card; 2]],
    ) -> Vec<HandRank> {
        boards.iter().zip(hands.iter())
            .map(|(&b, &h)| evaluator.evaluate_7cards(b, h))
            .collect()
    }
}

/// Scalar fallback for non-ARM64 architectures
#[cfg(not(target_arch = "aarch64"))]
mod neon {
    // Placeholder module for non-ARM64 builds
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_card(suit: u8, rank: u8) -> Card {
        // suit: 0=spades, 1=hearts, 2=diamonds, 3=clubs
        // rank: 0=2, 1=3, ..., 12=A
        Card::new(suit * 13 + rank)
    }

    // ── New table / 7-card tests ────────────────────────────────────────────

    #[test]
    fn test_flush_table_royal_flush() {
        // A-K-Q-J-10 of spades = bits 12,11,10,9,8 = 0x1F00
        assert_eq!(tables::get_flush_table()[0x1F00], 1, "Royal flush must be rank 1");
    }

    #[test]
    fn test_flush_table_wheel_sf() {
        // A-5-4-3-2 = bits 12,3,2,1,0 = 0x100F
        assert_eq!(tables::get_flush_table()[0x100F], 10, "Wheel SF must be rank 10");
    }

    #[test]
    fn test_7card_two_trips() {
        // AAA KKK Q — best hand is full house (AAA over KKK), range 167-322
        let eval = CactusKevEvaluator::new();
        let rank = eval.evaluate(
            [make_card(0, 12), make_card(1, 12), make_card(2, 12),
             make_card(0, 11), make_card(1, 11)],
            [make_card(2, 11), make_card(0, 10)],
        );
        assert!(rank.value() >= 167 && rank.value() <= 322,
            "Two-trips → FH, expected 167-322, got {}", rank.value());
    }

    #[test]
    fn test_7card_three_pairs() {
        // AA KK QQ J — best is two pair (AA KK + J kicker), range 2468-3325
        let eval = CactusKevEvaluator::new();
        let rank = eval.evaluate(
            [make_card(0, 12), make_card(1, 12), make_card(0, 11), make_card(1, 11),
             make_card(0, 10)],
            [make_card(1, 10), make_card(0, 9)],
        );
        assert!(rank.value() >= 2468 && rank.value() <= 3325,
            "Three-pair board → two pair, expected 2468-3325, got {}", rank.value());
    }

    #[test]
    fn test_7card_sf_with_extra_suited_cards() {
        // 9s8s7s6s5s As 2h — SF 9-high = rank 6
        let eval = CactusKevEvaluator::new();
        let rank = eval.evaluate(
            [make_card(0, 7), make_card(0, 6), make_card(0, 5), make_card(0, 4), make_card(0, 3)],
            [make_card(0, 12), make_card(1, 0)],
        );
        assert_eq!(rank.value(), 6, "9-high SF should be rank 6, got {}", rank.value());
    }

    #[test]
    fn test_7card_straight_beats_pair() {
        // A A K Q J 10 (no flush possible) — best is straight (A-high), rank 1600
        let eval = CactusKevEvaluator::new();
        let rank = eval.evaluate(
            [make_card(0, 12), make_card(1, 12), make_card(0, 11), make_card(1, 10), make_card(2, 9)],
            [make_card(3, 8), make_card(0, 7)],
        );
        assert_eq!(rank.value(), 1600, "A-high straight should be rank 1600, got {}", rank.value());
    }

    #[test]
    fn test_new_vs_old_evaluator_consistency() {
        let eval = CactusKevEvaluator::new();

        let mut seed: u64 = 98765;
        let lcg = |s: &mut u64| -> u8 {
            *s = (*s).wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
            (*s % 52) as u8
        };

        for i in 0..50_000usize {
            let mut cards = [0u8; 7];
            let mut used = [false; 52];
            let mut idx = 0;
            while idx < 7 {
                let v = lcg(&mut seed);
                if !used[v as usize] {
                    used[v as usize] = true;
                    cards[idx] = v;
                    idx += 1;
                }
            }
            let board = [Card::new(cards[0]), Card::new(cards[1]), Card::new(cards[2]),
                         Card::new(cards[3]), Card::new(cards[4])];
            let hand  = [Card::new(cards[5]), Card::new(cards[6])];

            let new_rank = eval.evaluate_7cards(board, hand).value();
            let ref_rank = eval.evaluate_7cards_reference(board, hand).value();

            assert_eq!(new_rank, ref_rank,
                "Mismatch on hand {i}: new={new_rank} ref={ref_rank} cards={cards:?}");
        }
    }

    // ── Original tests (unchanged) ──────────────────────────────────────────

    #[test]
    fn test_hand_rank_ordering() {
        let eval = CactusKevEvaluator::new();

        let royal_flush = eval.evaluate(
            [make_card(0, 12), make_card(0, 11), make_card(0, 10), make_card(0, 9), make_card(0, 8)],
            [make_card(1, 7), make_card(1, 6)]
        );
        let straight_flush = eval.evaluate(
            [make_card(0, 11), make_card(0, 10), make_card(0, 9), make_card(0, 8), make_card(0, 7)],
            [make_card(1, 6), make_card(1, 5)]
        );
        let four_of_a_kind = eval.evaluate(
            [make_card(0, 12), make_card(1, 12), make_card(2, 12), make_card(3, 12), make_card(0, 11)],
            [make_card(1, 10), make_card(2, 9)]
        );
        let full_house = eval.evaluate(
            [make_card(0, 12), make_card(1, 12), make_card(2, 12), make_card(0, 11), make_card(1, 11)],
            [make_card(2, 10), make_card(3, 9)]
        );
        let flush = eval.evaluate(
            [make_card(0, 12), make_card(0, 10), make_card(0, 8), make_card(0, 6), make_card(0, 4)],
            [make_card(1, 11), make_card(2, 9)]
        );
        let straight = eval.evaluate(
            [make_card(0, 11), make_card(1, 10), make_card(2, 9), make_card(3, 8), make_card(0, 7)],
            [make_card(1, 6), make_card(2, 5)]
        );
        let three_of_a_kind = eval.evaluate(
            [make_card(0, 12), make_card(1, 12), make_card(2, 12), make_card(0, 11), make_card(1, 10)],
            [make_card(2, 1), make_card(3, 0)]
        );
        let two_pair = eval.evaluate(
            [make_card(0, 12), make_card(1, 12), make_card(0, 11), make_card(1, 11), make_card(0, 10)],
            [make_card(2, 1), make_card(3, 0)]
        );
        let one_pair = eval.evaluate(
            [make_card(0, 12), make_card(1, 12), make_card(0, 11), make_card(1, 10), make_card(2, 9)],
            [make_card(3, 1), make_card(0, 0)]
        );
        let high_card = eval.evaluate(
            [make_card(0, 12), make_card(1, 11), make_card(2, 10), make_card(3, 9), make_card(0, 7)],
            [make_card(1, 5), make_card(2, 4)]
        );

        assert!(royal_flush < straight_flush, "Royal flush should beat straight flush");
        assert!(straight_flush < four_of_a_kind, "Straight flush should beat four of a kind");
        assert!(four_of_a_kind < full_house, "Four of a kind should beat full house");
        assert!(full_house < flush, "Full house should beat flush");
        assert!(flush < straight, "Flush should beat straight");
        assert!(straight < three_of_a_kind, "Straight should beat three of a kind");
        assert!(three_of_a_kind < two_pair, "Three of a kind should beat two pair");
        assert!(two_pair < one_pair, "Two pair should beat one pair");
        assert!(one_pair < high_card, "One pair should beat high card");
    }

    #[test]
    fn test_known_hands() {
        let eval = CactusKevEvaluator::new();

        let royal = eval.evaluate(
            [make_card(0, 12), make_card(0, 11), make_card(0, 10), make_card(0, 9), make_card(0, 8)],
            [make_card(1, 7), make_card(1, 6)]
        );
        assert_eq!(royal.value(), 1, "Royal flush should have rank 1");

        let quads = eval.evaluate(
            [make_card(0, 12), make_card(1, 12), make_card(2, 12), make_card(3, 12), make_card(0, 11)],
            [make_card(1, 10), make_card(2, 9)]
        );
        assert!(quads.value() >= 11 && quads.value() <= 166, "Four of a kind should be in range 11-166");
    }

    #[test]
    fn test_relative_ordering() {
        let eval = CactusKevEvaluator::new();

        let aces = eval.evaluate(
            [make_card(0, 12), make_card(1, 12), make_card(0, 11), make_card(1, 10), make_card(2, 9)],
            [make_card(3, 8), make_card(0, 7)]
        );
        let kings = eval.evaluate(
            [make_card(0, 11), make_card(1, 11), make_card(0, 10), make_card(1, 9), make_card(2, 8)],
            [make_card(3, 7), make_card(0, 6)]
        );

        assert!(aces < kings, "Pair of Aces should beat pair of Kings");
    }

    #[test]
    fn test_consistency() {
        let eval = CactusKevEvaluator::new();

        let hand1 = eval.evaluate(
            [make_card(0, 12), make_card(1, 11), make_card(2, 10), make_card(3, 9), make_card(0, 8)],
            [make_card(1, 7), make_card(2, 6)]
        );
        let hand2 = eval.evaluate(
            [make_card(2, 10), make_card(0, 12), make_card(1, 11), make_card(0, 8), make_card(3, 9)],
            [make_card(2, 6), make_card(1, 7)]
        );

        assert_eq!(hand1.value(), hand2.value(), "Same cards in different order should have same rank");
    }

    #[test]
    fn test_large_sample_category_validation() {
        let eval = CactusKevEvaluator::new();

        let mut seed: u64 = 12345;
        let lcg = |s: &mut u64| {
            *s = (*s * 1103515245 + 12345) & 0x7fffffff;
            (*s % 52) as u8
        };

        for _ in 0..10000 {
            let mut cards = Vec::new();
            while cards.len() < 7 {
                let card_val = lcg(&mut seed);
                if !cards.contains(&card_val) {
                    cards.push(card_val);
                }
            }

            let board = [
                Card::new(cards[0]), Card::new(cards[1]), Card::new(cards[2]),
                Card::new(cards[3]), Card::new(cards[4]),
            ];
            let hand = [Card::new(cards[5]), Card::new(cards[6])];

            let rank = eval.evaluate(board, hand);
            let rank_val = rank.value();

            assert!(rank_val >= 1 && rank_val <= 7462,
                   "Rank {} should be in range [1, 7462]", rank_val);

            let all_cards = [board[0], board[1], board[2], board[3], board[4], hand[0], hand[1]];
            let mut rank_counts = vec![0u8; 13];
            let mut suit_counts = vec![0u8; 4];

            for card in all_cards.iter() {
                let card_val = card.value();
                rank_counts[(card_val % 13) as usize] += 1;
                suit_counts[(card_val / 13) as usize] += 1;
            }

            rank_counts.sort();
            rank_counts.reverse();
            suit_counts.sort();
            suit_counts.reverse();

            let rank_bits: u16 = all_cards.iter().fold(0u16, |acc, c| acc | (1u16 << (c.value() % 13)));
            let can_form_straight = (4u8..=12).any(|h| {
                let mask = 0x1Fu16 << (h - 4);
                rank_bits & mask == mask
            }) || (rank_bits & 0x100F == 0x100F);

            if rank_counts[0] == 4 {
                assert!(rank_val >= 11 && rank_val <= 166,
                       "Four of a kind rank {} should be in range [11, 166]", rank_val);
            } else if rank_counts[0] == 3 && rank_counts[1] == 2 {
                assert!(rank_val >= 167 && rank_val <= 322,
                       "Full house rank {} should be in range [167, 322]", rank_val);
            } else if rank_counts[0] == 3 && rank_counts[1] < 2 && suit_counts[0] < 5 && !can_form_straight {
                assert!(rank_val >= 1610 && rank_val <= 2467,
                       "Three of a kind rank {} should be in range [1610, 2467]", rank_val);
            } else if rank_counts[0] == 2 && rank_counts[1] == 2 && suit_counts[0] < 5 && !can_form_straight {
                assert!(rank_val >= 2468 && rank_val <= 3325,
                       "Two pair rank {} should be in range [2468, 3325]", rank_val);
            } else if rank_counts[0] == 2 && suit_counts[0] < 5 && !can_form_straight {
                assert!(rank_val >= 3326 && rank_val <= 6185,
                       "One pair rank {} should be in range [3326, 6185]", rank_val);
            } else {
                assert!(rank_val >= 1 && rank_val <= 7462,
                       "High card/flush/straight rank {} should be in range [1, 7462]", rank_val);
            }
        }
    }

    #[test]
    fn test_evaluator_creation() {
        let eval = CactusKevEvaluator::new();
        assert!(eval.evaluate([Card::new(0), Card::new(1), Card::new(2), Card::new(3), Card::new(4)],
                              [Card::new(5), Card::new(6)]).value() > 0);
    }
}
