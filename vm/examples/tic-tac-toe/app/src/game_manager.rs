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
use crate::game::{Game, GameMove, Tile};
use crate::json_parser::{
    CreateGameResponse, CreatePlayerResponse, GetGameStateResponse, MoveResponse,
};
use crate::player::Player;

use arraydeque::{ArrayDeque, Wrapping};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

mod settings {
    pub const PLAYERS_MAX_COUNT: usize = 1024;
    pub const GAMES_MAX_COUNT: usize = 1024;
}

pub struct GameManager {
    players: ArrayDeque<[Rc<RefCell<Player>>; settings::PLAYERS_MAX_COUNT], Wrapping>,
    games: ArrayDeque<[Rc<RefCell<Game>>; settings::GAMES_MAX_COUNT], Wrapping>,
    players_by_name: HashMap<String, Weak<RefCell<Player>>>,
}

impl GameManager {
    pub fn new() -> Self {
        GameManager {
            players: ArrayDeque::new(),
            games: ArrayDeque::new(),
            players_by_name: HashMap::new(),
        }
    }

    pub fn make_move(
        &self,
        player_name: String,
        player_sign: String,
        coords: (usize, usize),
    ) -> AppResult<Value> {
        let game = self.get_player_game(player_name.clone(), player_sign)?;
        let game_move = GameMove::new(coords.0, coords.1)
            .ok_or_else(|| format!("Invalid coordinates: x = {} y = {}", coords.0, coords.1))?;

        let response = match game.borrow_mut().player_move(game_move)? {
            Some(app_move) => MoveResponse {
                player_name,
                winner: "None".to_owned(),
                coords: (app_move.x, app_move.y),
            },
            None => MoveResponse {
                player_name,
                winner: game.borrow_mut().get_winner().unwrap().to_string(),
                coords: (std::usize::MAX, std::usize::MAX),
            },
        };

        serde_json::to_value(response).map_err(Into::into)
    }

    pub fn create_player(&mut self, player_name: String, player_sign: String) -> AppResult<Value> {
        let new_player = Rc::new(RefCell::new(Player::new(player_name.clone(), player_sign)));
        self.players_by_name
            .insert(new_player.borrow().name.clone(), Rc::downgrade(&new_player));

        if let Some(prev) = self.players.push_back(new_player) {
            // if some elements poped from the deque, delete a corresponding weak link from
            // names_to_players
            self.players_by_name.remove(&prev.borrow().name);
        }

        let response = CreatePlayerResponse {
            player_name,
            result: "A new player has been successfully created".to_owned(),
        };
        serde_json::to_value(response).map_err(Into::into)
    }

    pub fn create_game(
        &mut self,
        player_name: String,
        player_sign: String,
        player_tile: Tile,
    ) -> AppResult<Value> {
        let player = self.get_player(player_name.clone(), player_sign)?;

        let game_state = Rc::new(RefCell::new(Game::new(player_tile)));
        player.borrow_mut().game = Rc::downgrade(&game_state);
        self.games.push_back(game_state);

        let response = CreateGameResponse {
            player_name,
            result: "A new game has been successfully created".to_owned(),
        };
        serde_json::to_value(response).map_err(Into::into)
    }

    pub fn get_game_state(&self, player_name: String, player_sign: String) -> AppResult<Value> {
        let game = self.get_player_game(player_name.clone(), player_sign)?;
        let (chosen_tile, board) = game.borrow().get_state();

        let response = GetGameStateResponse {
            player_name,
            player_tile: chosen_tile.to_char(),
            board,
        };
        serde_json::to_value(response).map_err(Into::into)
    }

    fn get_player(
        &self,
        player_name: String,
        player_sign: String,
    ) -> AppResult<Rc<RefCell<Player>>> {
        // try to find player by name in names_to_players and then convert Weak<Player> to Rc<Player>
        let player = match self.players_by_name.get(&player_name) {
            Some(player) => player.upgrade().ok_or_else(|| {
                "Internal error occurred - player has been already removed".to_owned()
            }),
            None => Err(format!(
                "Player with name {} and sign {} wasn't found",
                player_name, player_sign
            )),
        }?;

        // checks player signature
        if player.borrow().sign == player_sign {
            return Ok(player);
        }

        // errors are not distinguishable to not to give out the names of players
        Err(format!(
            "Player with name {} and sign {} wasn't found",
            player_name, player_sign
        ))
        .map_err(Into::into)
    }

    fn get_player_game(
        &self,
        player_name: String,
        player_sign: String,
    ) -> AppResult<Rc<RefCell<Game>>> {
        self.get_player(player_name, player_sign)?
            .borrow_mut()
            .game
            .upgrade()
            .ok_or_else(|| {
                "Sorry! Your game has been deleted, but you can start a new one".to_owned()
            })
            .map_err(Into::into)
    }
}
