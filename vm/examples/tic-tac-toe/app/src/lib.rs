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
mod game;
mod game_manager;
mod json_parser;
mod player;

use crate::error_type::AppResult;
use crate::game_manager::GameManager;
use crate::json_parser::*;

use fluence::sdk::*;
use serde_json::{json, Value};
use std::cell::RefCell;

mod settings {
    pub const PLAYERS_MAX_COUNT: usize = 1024;
    pub const GAMES_MAX_COUNT: usize = 1024;
    // to prevent DoS attack with large strings
    pub const USER_NAME_MAX_LEN: usize = 1024;
}

thread_local! {
    static GAME_MANAGER: RefCell<GameManager> = RefCell::new(GameManager::new());
}

fn do_request(req: String) -> AppResult<Value> {
    let raw_request: Value = serde_json::from_str(req.as_str())?;
    let request: Request = serde_json::from_value(raw_request.clone())?;

    match request.action.as_str() {
        "move" => {
            let player_move: PlayerMove = serde_json::from_value(raw_request)?;
            GAME_MANAGER.with(|gm| {
                gm.borrow()
                    .make_move(request.player_name, player_move.coords)
            })
        }

        "create_player" => {
            GAME_MANAGER.with(|gm| gm.borrow_mut().create_player(request.player_name))
        }

        "create_game" => {
            let player_tile: PlayerTile = serde_json::from_value(raw_request)?;
            let player_tile = game::Tile::from_char(player_tile.tile).ok_or_else(|| {
                "incorrect tile type, please choose it from {'X', 'O'} set".to_owned()
            })?;

            GAME_MANAGER.with(|gm| {
                gm.borrow_mut()
                    .create_game(request.player_name, player_tile)
            })
        }

        "get_game_state" => GAME_MANAGER.with(|gm| gm.borrow().get_game_state(request.player_name)),

        "get_statistics" => GAME_MANAGER.with(|gm| gm.borrow().get_statistics()),

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
