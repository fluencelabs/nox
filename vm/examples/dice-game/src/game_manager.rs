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

use crate::error_type::AppResult;
use crate::request_response::Response;

use crate::settings::{INIT_ACCOUNT_BALANCE, PAYOUT_RATE, PLAYERS_MAX_COUNT, SEED};
use linked_hash_map::LinkedHashMap;
use rand::{Rng, SeedableRng};
use rand_isaac::IsaacRng;
use serde_json::Value;

pub struct GameManager {
    // map from players id to account state
    players: LinkedHashMap<u64, u64>,
    // count of registered players, used for new player id generation
    registered_players: u64,
    // random generator, used for generating dice result
    rng: IsaacRng,
}

impl GameManager {
    pub const DICE_LINE_COUNT: u8 = 6;

    pub fn new() -> Self {
        GameManager {
            registered_players: 0,
            players: LinkedHashMap::new(),
            rng: SeedableRng::seed_from_u64(SEED),
        }
    }

    /// Creates a new player, returns its id.
    pub fn join(&mut self) -> AppResult<Value> {
        if self.players.len() >= PLAYERS_MAX_COUNT {
            self.players.pop_front();
        }

        self.players
            .insert(self.registered_players, INIT_ACCOUNT_BALANCE);

        let response = Response::Join {
            player_id: self.registered_players,
        };

        self.registered_players += 1;

        serde_json::to_value(response).map_err(Into::into)
    }

    /// Checks parameters of given bet and processes it.
    pub fn bet(&mut self, player_id: u64, placement: u8, bet_amount: u32) -> AppResult<Value> {
        fn check_bet(player_balance: u64, placement: u8, bet_amount: u64) -> AppResult<()> {
            if bet_amount > player_balance {
                return Err(format!(
                    "Player hasn't enough money: player's current balance is {} while the bet is {}",
                    player_balance, bet_amount
                ))
                    .map_err(Into::into);
            }

            if placement > GameManager::DICE_LINE_COUNT {
                return Err("Incorrect placement, please choose number from 1 to 6")
                    .map_err(Into::into);
            }

            Ok(())
        }

        fn update_balance(player_balance: u64, bet_amount: u64, placement: u8, outcome: u8) -> u64 {
            if placement == outcome {
                player_balance + (bet_amount * PAYOUT_RATE)
            } else {
                player_balance - bet_amount
            }
        }

        let player_balance = self.player_balance(player_id)?;
        let bet_amount = u64::from(bet_amount);
        check_bet(player_balance, placement, bet_amount)?;

        let outcome = self.rng.gen::<u8>() % GameManager::DICE_LINE_COUNT + 1;
        let new_player_balance = update_balance(player_balance, bet_amount, placement, outcome);

        let response = Response::Bet {
            outcome,
            player_balance: new_player_balance,
        };

        // update balance of the player
        *self.players.get_mut(&player_id).unwrap() = new_player_balance;

        serde_json::to_value(response).map_err(Into::into)
    }

    /// Returns the balance of the player identified by given `player_id`.
    pub fn get_player_balance(&self, player_id: u64) -> AppResult<Value> {
        let player_balance = self.player_balance(player_id)?;
        let response = Response::GetBalance { player_balance };

        serde_json::to_value(response).map_err(Into::into)
    }

    // returns a balance if there is a such player and Err() otherwise
    fn player_balance(&self, player_id: u64) -> AppResult<u64> {
        let balance = self
            .players
            .get(&player_id)
            .ok_or_else(|| format!("Player with id {} wasn't found", player_id))?;

        Ok(*balance)
    }
}
