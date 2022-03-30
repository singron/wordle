#![feature(stdsimd)]
use std::fmt::Display;
use std::io::{BufRead, BufReader};
use std::num::NonZeroU8;

use rayon::prelude::*;

/// A 5 letter word. Each byte is an ascii lowercase letter a-z.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Word(pub [u8; 5]);

/// A Word packed into a u64.
struct Word64(u64);

impl Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::str::from_utf8(&self.0).unwrap().fmt(f)
    }
}

impl Word {
    fn to_u64(self) -> Word64 {
        let u = self.0[0] as u64
            + ((self.0[1] as u64) << 8)
            + ((self.0[2] as u64) << 16)
            + ((self.0[3] as u64) << 24)
            + ((self.0[4] as u64) << 32);
        Word64(u)
    }
}

pub fn read_wordlist(path: &str) -> Vec<Word> {
    let f = std::fs::File::open(path).unwrap();
    let mut words = Vec::new();
    'line: for line in BufReader::new(f).lines() {
        let line = line.unwrap();
        if line.is_empty() {
            continue;
        }
        if line.len() != 5 {
            continue;
        }
        for c in line.chars() {
            if !c.is_ascii_lowercase() {
                continue 'line;
            }
        }
        words.push(Word(line.as_bytes().try_into().unwrap()));
    }
    words.sort();
    words
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct BitField(u8);

impl BitField {
    fn get(self, idx: usize) -> bool {
        debug_assert!(idx < 8);
        let idx = idx as u8;
        self.0 & (1 << idx) != 0
    }

    fn set(&mut self, idx: usize) {
        debug_assert!(idx < 8);
        let idx = idx as u8;
        self.0 |= 1 << idx;
    }

    #[allow(unused)]
    fn unset(&mut self, idx: usize) {
        debug_assert!(idx < 8);
        let idx = idx as u8;
        self.0 &= (1 << idx) ^ 0xff;
    }
}

#[derive(Clone)]
struct CharInfo {
    // Bit 0-4: What fields do NOT have the char.
    // Bit 7: Is the char NOT in the word (i.e. out).
    neg: BitField,
    // Bit 0-4: What fields DO have the char.
    // Bit 7: Is the char IN the word.
    pos: BitField,
}

impl CharInfo {
    const UNKNOWN: CharInfo = CharInfo {
        neg: BitField(0),
        pos: BitField(0),
    };

    fn deduce(&mut self) {
        if self.is_in() {
            // If the char is in the word, and it's not in 4 positions, it must be in the 5th
            // position.
            let mut negs = 0;
            for i in 0..5 {
                if self.neg.get(i) {
                    negs += 1;
                }
            }
            if negs == 4 {
                for i in 0..5 {
                    if !self.neg.get(i) {
                        self.pos.set(i);
                    }
                }
            }
        }
    }

    fn is_in(&self) -> bool {
        self.pos.get(7)
    }

    fn is_out(&self) -> bool {
        self.neg.get(7)
    }

    fn set_in(&mut self) {
        debug_assert!(!self.is_out());
        self.pos.set(7);
    }

    fn set_out(&mut self) {
        debug_assert!(!self.is_in());
        self.neg.set(7)
    }
}

#[derive(Clone)]
pub struct Game {
    positions: [Option<NonZeroU8>; 5],
    // Bits set if we know the char in position_packed
    position_mask: u16,
    position_packed: u64,
    chars: [CharInfo; 26],
    pub guesses: u32,
}

impl Game {
    pub fn new() -> Game {
        Game {
            positions: [None; 5],
            position_mask: 0,
            position_packed: 0,
            chars: [CharInfo::UNKNOWN; 26],
            guesses: 0,
        }
    }

    fn is_possible(&self, w: Word) -> bool {
        // This is the hot code in the program. Parts of this have simd implementations.

        // Must have Green characters in the right places.
        if cfg!(debug_assertions) {
            let mut green_good = true;
            for (i, c) in self.positions.iter().enumerate() {
                if let Some(c) = c {
                    if w.0[i] != (*c).into() {
                        green_good = false;
                        break;
                    }
                }
            }
            debug_assert_eq!(green_good, self.is_possible_simd_green(w.to_u64()));
            if !green_good {
                return false;
            }
        } else {
            if !self.is_possible_simd_green(w.to_u64()) {
                return false;
            }
        }

        // Can't use characters we know aren't there
        if cfg!(debug_assertions) {
            let mut no_bad = true;
            for i in 0..5 {
                let c = w.0[i];
                let idx = (c - b'a') as usize;
                let info = &self.chars[idx];
                if info.is_out() || info.neg.get(i) {
                    no_bad = false;
                    break;
                }
            }
            debug_assert_eq!(no_bad, self.is_possible_simd_no_bad(w.to_u64()));
            if !no_bad {
                return false;
            }
        } else {
            if !self.is_possible_simd_no_bad(w.to_u64()) {
                return false;
            }
        }
        // Must have Yellow characters.
        for (idx, info) in self.chars.iter().enumerate() {
            if info.is_in() {
                let c = b'a' + idx as u8;
                if !w.0.contains(&c) {
                    return false;
                }
            }
        }
        true
    }

    fn is_possible_simd_green(&self, w: Word64) -> bool {
        use std::arch::asm;
        use std::arch::x86_64::*;
        let wv = unsafe { _mm_set_epi64x(0, w.0 as i64) };
        let mut pv = unsafe { _mm_set_epi64x(0, self.position_packed as i64) };
        unsafe {
            asm!("pcmpeqb {}, {}", inout(xmm_reg) pv, in(xmm_reg) wv);
        }
        let res = unsafe { _mm_movemask_epi8(pv) } as u16;
        // Invert for neq and mask.
        let cmp_mask = (0xffff ^ res) & self.position_mask;
        if cmp_mask != 0 {
            return false;
        }
        true
    }

    fn is_possible_simd_no_bad(&self, w: Word64) -> bool {
        use std::arch::x86_64::*;
        let wv = unsafe { _mm_set_epi64x(0, w.0 as i64) };
        let av = unsafe { _mm_set1_epi8(b'a' as i8) };
        // expanded to i32x4 to match gather
        let idx_v = unsafe { _mm_cvtepu8_epi32(_mm_sub_epi8(wv, av)) };
        let base = (&self.chars[0].neg.0) as *const u8;
        let _: &CharInfo = &self.chars[0];
        const SIZE: i32 = std::mem::size_of::<CharInfo>() as i32;
        let neg_v = unsafe { _mm_i32gather_epi32::<SIZE>(base as *const i32, idx_v) };
        let anyout_z =
            unsafe { _mm_testz_si128(_mm_set_epi32(1 << 7, 1 << 7, 1 << 7, 1 << 7), neg_v) } as u16;
        if anyout_z == 0 {
            return false;
        }
        // Figure out if the ith bit is set.
        let i_v = unsafe { _mm_set_epi32(1 << 3, 1 << 2, 1 << 1, 1 << 0) };
        // AND i. negi_z is 1 if all bits are 0 after the AND.
        let negi_z = unsafe { _mm_testz_si128(neg_v, i_v) };
        if negi_z != 1 {
            return false;
        }

        {
            // gather is only 4-wide, so we do the 5th char by itself
            let i = 4;
            let c = (w.0 >> 32) as u8;
            let idx = (c - b'a') as usize;
            let info = &self.chars[idx];
            if info.is_out() || info.neg.get(i) {
                return false;
            }
        }
        true
    }

    /// Guessing this word could give new information.
    fn is_revealing(&self, w: Word) -> bool {
        for i in 0..5 {
            let c = w.0[i];
            let idx = (c - b'a') as usize;
            let info = &self.chars[idx];
            if info.is_out() {
                // Can't learn more about this char.
                continue;
            }
            if !info.is_in() {
                return true;
            }
            if !info.neg.get(i) && !info.pos.get(i) {
                // We don't know about this position.
                return true;
            }
        }
        false
    }

    pub fn guess(&mut self, answer: Word, w: Word) {
        self.guesses += 1;
        for i in 0..5 {
            let c = w.0[i];
            let idx = (c - b'a') as usize;
            let info = &mut self.chars[idx];
            if w.0[i] == answer.0[i] {
                self.positions[i] = Some(w.0[i].try_into().unwrap());
                self.position_mask |= 1 << i;
                self.position_packed |= (w.0[i] as u64) << (i * 8);
                info.set_in();
                info.pos.set(i);
            } else {
                info.neg.set(i);
            }
            if answer.0.contains(&c) {
                info.set_in();
            } else {
                info.set_out();
            }
        }

        for c in w.0 {
            let idx = (c - b'a') as usize;
            let info = &mut self.chars[idx];
            info.deduce();
        }
    }
}

/// Pick a guess that minimizes the maximum number of possible answers (i.e. minimax).
pub fn best_guess(game: &Game, answer_words: &[Word], guess_words: &[Word]) -> Word {
    // Words won't become possible, so pre-filter the current game state.
    let possible_words: Vec<Word> = answer_words
        .iter()
        .filter(|&&w| game.is_possible(w))
        .map(|&w| w)
        .collect();
    if possible_words.len() == 1 {
        // Ocassionally, we are able to deduce the final word. The below loop won't terminate
        // because the only remaining word isn't able to reveal additional information.
        return possible_words[0];
    }
    // Use all words since we can still guess a word even if we know it can't be an answer.
    *guess_words
        .par_iter()
        .filter(|&&w| game.is_revealing(w))
        .min_by_key(|&&guess| {
            let mut max_answers = 0;
            for &a in &possible_words {
                let mut game = game.clone();
                game.guess(a, guess);
                let mut answers = 0;
                for &na in &possible_words {
                    if !game.is_possible(na) {
                        continue;
                    }
                    answers += 1;
                }
                if answers > max_answers {
                    max_answers = answers;
                }
            }
            max_answers
        })
        .unwrap()
}
