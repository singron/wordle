use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wordle::*;

fn first_guess(words: &[Word]) -> Word {
    let mut game = Game::new();
    best_guess(&mut game, words, words)
}

fn bench_best_guess(c: &mut Criterion) {
    c.bench_function("guess_100", |b| {
        let mut words = read_wordlist("answer_words.txt");
        words.sort();
        let words = &words[0..100];
        b.iter(|| first_guess(black_box(&words)));
    });
}

criterion_group!(benches, bench_best_guess);
criterion_main!(benches);
