//! Benchmark harness for hand evaluator throughput
//!
//! Measures 7-card evaluation performance with the goal of achieving
//! 50M+ evals/sec on Apple Silicon with NEON batching.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oracle_engine::evaluator::CactusKevEvaluator;
use oracle_engine::HandEvaluator;
use oracle_engine::node::Card;

/// Simple LCG for deterministic random number generation
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = (self.state.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff;
        self.state
    }

    fn next_card(&mut self) -> Card {
        Card::new((self.next() % 52) as u8)
    }
}

/// Generate a batch of random (board, hand) pairs
fn generate_test_hands(count: usize, seed: u64) -> Vec<([Card; 5], [Card; 2])> {
    let mut lcg = Lcg::new(seed);
    let mut hands = Vec::with_capacity(count);

    for _ in 0..count {
        // Generate 7 unique cards
        let mut cards = Vec::new();
        while cards.len() < 7 {
            let card = lcg.next_card();
            if !cards.contains(&card) {
                cards.push(card);
            } else {
                // If duplicate, try next card
                lcg.next();
            }
        }

        let board = [cards[0], cards[1], cards[2], cards[3], cards[4]];
        let hand = [cards[5], cards[6]];
        hands.push((board, hand));
    }

    hands
}

fn benchmark_scalar_evaluation(c: &mut Criterion) {
    let evaluator = CactusKevEvaluator::new();
    let test_hands = generate_test_hands(1_000_000, 12345);

    c.bench_function("hand_evaluator_7card_scalar", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for (board, hand) in black_box(&test_hands) {
                let rank = evaluator.evaluate(*board, *hand);
                sum += rank.value() as u64;
            }
            black_box(sum)
        })
    });
}

fn benchmark_batch_evaluation(c: &mut Criterion) {
    let evaluator = CactusKevEvaluator::new();
    let test_hands = generate_test_hands(1_000_000, 12345);
    
    let boards: Vec<[Card; 5]> = test_hands.iter().map(|(b, _)| *b).collect();
    let hands: Vec<[Card; 2]> = test_hands.iter().map(|(_, h)| *h).collect();

    c.bench_function("hand_evaluator_7card_batch", |b| {
        b.iter(|| {
            let results = evaluator.evaluate_batch(black_box(&boards), black_box(&hands));
            black_box(results.len())
        })
    });
}

criterion_group!(benches, benchmark_scalar_evaluation, benchmark_batch_evaluation);
criterion_main!(benches);
