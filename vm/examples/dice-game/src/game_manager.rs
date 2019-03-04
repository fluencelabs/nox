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
use crate::json_parser::{BetResponse, GetBalanceResponse, JoinResponse};

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
    pub fn new() -> Self {
        GameManager {
            registered_players: 0,
            players: LinkedHashMap::new(),
            rng: SeedableRng::seed_from_u64(SEED),
        }
    }

    /// Marks an empty position on the board by user's tile type. Returns MoveResponse structure
    /// as a serde_json Value.
    pub fn join(&mut self) -> AppResult<Value> {
        if self.players.len() >= PLAYERS_MAX_COUNT {
            self.players.pop_front();
        }

        self.players
            .insert(self.registered_players, INIT_ACCOUNT_BALANCE);

        let response = JoinResponse {
            player_id: self.registered_players,
        };

        self.registered_players += 1;
        Ok(serde_json::to_value(response).unwrap())
    }

    /// Creates a new player with given player name.
    pub fn bet(&mut self, player_id: u64, placement: u8, bet_amount: u32) -> AppResult<Value> {
        let player_balance: u64 = self.player_balance(player_id)?;

        if bet_amount as u64 > player_balance {
            return Err(format!(
                "Player {} hasn't enough money: player's current balance is {} while the bet is {}",
                player_id, player_balance, bet_amount
            ))
            .map_err(Into::into);
        }

        let dice_lines_count = 6;
        if placement > dice_lines_count {
            return Err("Incorrect placement, please choose number from 1 to 6").map_err(Into::into);
        }

        let outcome = self.rng.gen::<u8>() % dice_lines_count + 1;

        let new_player_balance = if placement == outcome {
            player_balance + (bet_amount * PAYOUT_RATE) as u64
        } else {
            player_balance - bet_amount as u64
        };

        let response = BetResponse {
            outcome,
            player_balance: new_player_balance,
        };

        *self.players.get_mut(&player_id).unwrap() = new_player_balance;

        Ok(serde_json::to_value(response).unwrap())
    }

    pub fn get_player_balance(&self, player_id: u64) -> AppResult<Value> {
        let player_balance = self.player_balance(player_id)?;

        let response = GetBalanceResponse { player_balance };

        Ok(serde_json::to_value(response).unwrap())
    }

    fn player_balance(&self, player_id: u64) -> AppResult<u64> {
        let balance = self
            .players
            .get(&player_id)
            .ok_or_else(|| format!("player with id {} wan't found", player_id))?;

        Ok(*balance)
    }
}
