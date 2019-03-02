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

//! A simple demo application for Fluence.
use fluence::sdk::*;
use log::info;
use rand::Rng;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use rand::{Rng, SeedableRng, StdRng};



thread_local! {
//    static GAME_MANAGER: RefCell<GameManager> = RefCell::new(GameManager::new());
    static mut GAMES_COUNT: u32 = 0;
    static mut SEED: u64 = 123456789;
    static mut SECRET: u8 = 0;
}


fn init() {
    logger::WasmLogger::init_with_level(log::Level::Info).unwrap();
}

#[invocation_handler(init_fn = init)]
fn game(input: String) -> String {
    update_seed(input);
}

fn update_seed(input: String) {
    let mut hasher = DefaultHasher::new();
    hasher.write(input.as_bytes());
    hasher.write_u32(GAMES_COUNT);
    hasher.write(SEED);
    SEED = hasher.finish();
}

fn gen_secret() -> u8 {
    let mut rng: StdRng = SeedableRng::from_seed(SEED);
}

fn greeting(name: String) -> String {
    let answer = format!("Hello, {}! Please, input your guess");
    info!(answer);
    answer
}
