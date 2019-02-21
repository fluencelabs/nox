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
use std::{cell::RefCell, collections::HashMap, rc::Rc, rc::Weak};

mod settings {
    pub const PLAYERS_MAX_COUNT: usize = 1024;
    pub const GAMES_MAX_COUNT: usize = 1024;
}

pub struct GameManager {
    players: ArrayDeque<[Rc<RefCell<Player>>; settings::PLAYERS_MAX_COUNT], Wrapping>,
    games: ArrayDeque<[Rc<RefCell<Game>>; settings::GAMES_MAX_COUNT], Wrapping>,
    // TODO: String key should be replaced with Cow<'a, str>. After that signatures of all public
    // functions also should be changed similar to https://jwilm.io/blog/from-str-to-cow/.
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

    /// Marks an empty position on the board by user's tile type. Returns MoveResponse structure
    /// as a serde_json Value.
    pub fn make_move(
        &self,
        player_name: String,
        player_sign: String,
        coords: (usize, usize),
    ) -> AppResult<Value> {
        let game = self.get_player_game(&player_name, &player_sign)?;
        let mut game = game.borrow_mut();
        let game_move = GameMove::new(coords.0, coords.1)
            .ok_or_else(|| format!("Invalid coordinates: x = {} y = {}", coords.0, coords.1))?;

        let response = match game.player_move(game_move)? {
            Some(app_move) => {
                // checks did the app win in this turn?
                let winner = match game.get_winner() {
                    Some(winner) => winner.to_string(),
                    None => "None".to_owned(),
                };
                MoveResponse {
                    player_name,
                    winner,
                    coords: (app_move.x, app_move.y),
                }
            }
            // none means a win of the player or a draw
            None => MoveResponse {
                player_name,
                winner: game.get_winner().unwrap().to_string(),
                coords: (std::usize::MAX, std::usize::MAX),
            },
        };

        serde_json::to_value(response).map_err(Into::into)
    }

    /// Creates a new player with given player name and player signature.
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

    /// Creates a new game for provided player. Note that the previous one is deleted (if it
    /// present) and won't be accessed anymore. Returns CreateGameResponse as a serde_json Value if
    /// 'X' tile type has been chosen and MoveResponse otherwise.
    pub fn create_game(
        &mut self,
        player_name: String,
        player_sign: String,
        player_tile: Tile,
    ) -> AppResult<Value> {
        let player = self.get_player(&player_name, &player_sign)?;

        let game_state = Rc::new(RefCell::new(Game::new(player_tile)));
        player.borrow_mut().game = Rc::downgrade(&game_state);

        let response = match player_tile {
            Tile::X => {
                let response = CreateGameResponse {
                    player_name,
                    result: "A new game has been successfully created".to_owned(),
                };
                serde_json::to_value(response)
            }
            // if the user chose 'O' tile the app should move first
            Tile::O => {
                let app_move = game_state.borrow_mut().app_move().unwrap();
                let response = MoveResponse {
                    player_name,
                    winner: "None".to_owned(),
                    coords: (app_move.x, app_move.y),
                };
                serde_json::to_value(response)
            }
        };

        self.games.push_back(game_state);
        response.map_err(Into::into)
    }

    /// Returns current game state for provided user as a GetGameStateResponse serde_json Value.
    pub fn get_game_state(&self, player_name: String, player_sign: String) -> AppResult<Value> {
        let game = self.get_player_game(&player_name, &player_sign)?;
        let (chosen_tile, board) = game.borrow().get_state();

        let response = GetGameStateResponse {
            player_name,
            player_tile: chosen_tile.to_char(),
            board,
        };
        serde_json::to_value(response).map_err(Into::into)
    }

    fn get_player(&self, player_name: &str, player_sign: &str) -> AppResult<Rc<RefCell<Player>>> {
        // try to find player by name in players_by_name and then convert Weak<Player> to Rc<Player>
        let player = match self.players_by_name.get(&player_name.to_owned()) {
            Some(player) => player.upgrade().ok_or_else(|| {
                "Internal error occurred - player has been already removed".to_owned()
            }),
            None => Err(format!(
                "Player with name {} and sign {} wasn't found",
                player_name, player_sign
            )),
        }?;

        // checks a player signature
        if player.borrow().sign == player_sign {
            return Ok(player);
        }

        // `auth` error and `player not found` error shouldn't be distinguishable to not give a
        // possibility to brute force players name (like in a real app)
        Err(format!(
            "Player with name {} and sign {} wasn't found",
            player_name, player_sign
        ))
        .map_err(Into::into)
    }

    fn get_player_game(
        &self,
        player_name: &str,
        player_sign: &str,
    ) -> AppResult<Rc<RefCell<Game>>> {
        self.get_player(player_name, player_sign)?
            // borrow a mutable link to Weak<RefCell<Game>>
            .borrow_mut()
            .game
            // tries to upgrade Weak<RefCell<Game>> to Rc<RefCell<Game>>
            .upgrade()
            .ok_or_else(|| "Sorry! Your game was deleted, but you can start a new one".to_owned())
            .map_err(Into::into)
    }
}
