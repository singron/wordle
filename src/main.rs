#![feature(stdsimd)]

use wordle::*;

fn play(answer: Word, answer_words: &[Word], guess_words: &[Word]) -> u32 {
    let mut game = Game::new();
    // First guess takes a long time to compute since every word is available, but since it doesn't
    // depend on the answer, it's always aesir.
    let aesir = Word(*b"aesir");
    for &w in guess_words {
        if w == aesir {
            //println!("{}: guess {}", answer, w);
            game.guess(answer, w);
            if w == answer {
                return game.guesses;
            }
            break;
        }
    }
    loop {
        let w = best_guess(&game, answer_words, guess_words);
        //println!("{}: guess {}", answer, w);
        game.guess(answer, w);
        if w == answer {
            return game.guesses;
        }
    }
}

/// Calculate the first guess. This takes a long time to run since the choice is unconstrained. The
/// result is hardcoded into play.
#[allow(unused)]
fn find_best_first_guess(answer_words: &[Word], guess_words: &[Word]) -> Word {
    let game = Game::new();
    best_guess(&game, answer_words, guess_words)
}

fn main() {
    // These words can be the final solution of the puzzle.
    let answer_words = read_wordlist("answer_words.txt");
    // These words can be guessed.
    let guess_words = read_wordlist("guess_words.txt");

    // Run against every possible answer and calculate some statistics.
    let mut min_guesses = 9999;
    let mut max_guesses = 0;
    let mut sum: u64 = 0;
    let mut wins: u64 = 0;
    for &answer in &answer_words {
        let guesses = play(answer, &answer_words, &guess_words);
        println!("{}: {}", answer, guesses);
        max_guesses = max_guesses.max(guesses);
        min_guesses = min_guesses.min(guesses);
        sum += guesses as u64;
        if guesses <= 6 {
            wins += 1;
        }
    }
    let avg = sum as f64 / answer_words.len() as f64;
    let win = wins as f64 / answer_words.len() as f64;
    println!(
        "Words={} Max={} Min={} Avg={:.2} Win={:.2}%",
        answer_words.len(),
        max_guesses,
        min_guesses,
        avg,
        win * 100.0
    );
}
