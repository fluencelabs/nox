/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use fluence::sdk::*;
use rand::{Rng, SeedableRng};
use rand_isaac::IsaacRng;

enum Guess {
    LESS,
    GREATER,
    EQUAL,
}

thread_local! {
    /// Number of games won, used for SEED generation
    static GAMES_COUNT: RefCell<u32> = RefCell::new(0);
    /// Number of tries, used for SEED generation
    static TRIES: RefCell<u32> = RefCell::new(0);
    /// Seed for rng
    static SEED: RefCell<u64> = RefCell::new(123456789);
    /// Number to guess
    static SECRET: RefCell<u8> = RefCell::new(0);
}

fn init() {
    /// Generate a secret
    update_secret();
}

#[invocation_handler(init_fn = init)]
fn game(input: String) -> String {
    count_request(&input);
    update_seed(&input);

    match input.parse::<i16>() {
        Err(e) => format!("Input can't be parsed as i16 {}: {}", input, e),
        Ok(guess) => match compare_guess(guess) {
            Guess::LESS => format!("Your guess is too low! Try something bigger."),
            Guess::GREATER => format!("Too big! Try again."),
            Guess::EQUAL => {
                next_game();
                format!("Success! You guessed right.")
            }
        },
    }
}

/// Increment number of tries
fn count_request(input: &String) {
    TRIES.with(|t| *t.borrow_mut() += 1);
}

/// Update seed for rng
fn update_seed(input: &String) {
    GAMES_COUNT.with(|count| {
        SEED.with(|seed| {
            let count = *count.borrow();
            let mut seed = seed.borrow_mut();
            let mut hasher = DefaultHasher::new();
            hasher.write(input.as_bytes());
            hasher.write_u32(count);
            hasher.write_u64(*seed);
            *seed = hasher.finish();
        })
    })
}

/// Update secret from rng(SEED)
fn update_secret() {
    SEED.with(|seed| {
        let mut rng: IsaacRng = SeedableRng::seed_from_u64(*seed.borrow());
        SECRET.with(|secret| {
            *secret.borrow_mut() = rng.gen::<u8>();
        })
    })
}

/// Compare user's guess to SECRET
fn compare_guess(guess: i16) -> Guess {
    SECRET.with(|secret| {
        let secret = *secret.borrow() as i16;
        match guess {
            _ if guess < secret => Guess::LESS,
            _ if guess > secret => Guess::GREATER,
            _ => Guess::EQUAL,
        }
    })
}

/// Generate next secret and increment won games count
fn next_game() {
    GAMES_COUNT.with(|c| *c.borrow_mut() += 1);
    update_secret();
}

#[test]
fn test_low() {
    let expected_answer = String::from("Your guess is too low! Try something bigger.");
    let answer = game("-1".to_string());
    assert_eq!(answer, expected_answer);
}

#[test]
fn test_high() {
    let expected_answer = String::from("Too big! Try again.");
    let answer = game(i16::max_value().to_string());
    assert_eq!(answer, expected_answer);
}

#[test]
fn test_eq() {
    let value: u8 = 10;
    SECRET.with(|secret| *secret.borrow_mut() = value);
    let expected_answer = String::from("Success! You guessed right.");
    let answer = game(value.to_string());
    assert_eq!(answer, expected_answer);
}
