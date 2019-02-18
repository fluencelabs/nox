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

mod player;
mod game;
mod game_manager;

use crate::game_manager::GameManager;
use fluence::sdk::*;
use log::{info, error};
use serde_json::{Result, Value};
use std::cell::Cell;
use std::cell::RefCell;
use std::thread;

thread_local! {
    static GAME_MANAGER: RefCell<GameManager> = RefCell::new(GameManager::new());
}

// initializes logger
fn init() {
    logger::WasmLogger::init_with_level(log::Level::Info).unwrap();
    info!("initialize");
}

fn do_request(req: String) -> String {
    let json: Value = match serde_json::from_str(&req[..]) {
        Ok(res) => res,
        Err(err) => {
            let err_message = format!("error while parsing {} json", err);
            error!("{}", err_message);
            return err_message;
        }
    };

    let action = json["action"].as_str();
    match action {
        None => {
            let err_message = format!("There is no action key in json");
            error!("{}", err_message);
            err_message
        },
        Some(s) => {
            let player_name = json["player_name"].as_str().unwrap();
            let player_sign = json["player_sign"].as_str().unwrap();
            match s {
                "move" =>
                    GAME_MANAGER.with(|gm|
                        gm.borrow().make_move(player_name.to_owned(), player_sign.to_owned())
                    ),
                "create_player" =>
                    GAME_MANAGER.with(|gm|
                        gm.borrow_mut().create_player(player_name.to_owned(), player_sign.to_owned())
                    ),
                "create_game" =>
                    GAME_MANAGER.with(|gm|
                        gm.borrow_mut().create_game(player_name.to_owned(), player_sign.to_owned())
                    ),
                "get_game_state" =>
                    GAME_MANAGER.with(|gm|
                        gm.borrow().get_game_state(player_name.to_owned(), player_sign.to_owned())
                    ),
                _ => {
                    let err_message = format!("{} action key is unsupported", s);
                    error!("{}", err_message);
                    err_message
                },
            }
        }
    }
}

#[invocation_handler(init_fn = init)]
fn greeting(req: String) -> String {
//    info!("the new request {}", req);
    do_request(req)
}
