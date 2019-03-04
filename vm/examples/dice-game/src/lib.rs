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

#[cfg(test)]
mod tests;

mod error_type;
mod game_manager;
mod json_parser;

use crate::error_type::AppResult;
use crate::game_manager::GameManager;
use crate::json_parser::*;

use fluence::sdk::*;
use serde_json::{json, Value};
use std::cell::RefCell;

mod settings {
    pub const PLAYERS_MAX_COUNT: usize = 1024;
    pub const SEED: u64 = 12345678;
    // the account balance of new players
    pub const INIT_ACCOUNT_BALANCE: u64 = 100;
    // if win, player receives PAYOUT_RATE*bet money
    pub const PAYOUT_RATE: u32 = 5;
}

thread_local! {
    static GAME_MANAGER: RefCell<GameManager> = RefCell::new(GameManager::new());
}

fn do_request(req: String) -> AppResult<Value> {
    let raw_request: Value = serde_json::from_str(req.as_str())?;
    let request: Request = serde_json::from_value(raw_request.clone())?;

    match request.action.as_str() {
        "join" => GAME_MANAGER.with(|gm| gm.borrow_mut().join()),

        "bet" => {
            let bet_request: BetRequest = serde_json::from_value(raw_request)?;
            GAME_MANAGER.with(|gm| {
                gm.borrow_mut().bet(
                    bet_request.player_id,
                    bet_request.placement,
                    bet_request.bet_amount,
                )
            })
        }

        "get_account_state" => {
            let get_balance_request: GetBalanceRequest = serde_json::from_value(raw_request)?;
            GAME_MANAGER.with(|gm| {
                gm.borrow_mut()
                    .get_player_balance(get_balance_request.player_id)
            })
        }

        _ => Err(format!("{} action key is unsupported", request.action)).map_err(Into::into),
    }
}

#[invocation_handler]
fn main(req: String) -> String {
    match do_request(req) {
        Ok(req) => req.to_string(),
        Err(err) => json!({
            "error": err.to_string()
        })
        .to_string(),
    }
}
