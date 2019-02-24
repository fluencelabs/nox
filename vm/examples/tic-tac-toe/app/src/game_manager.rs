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
    CreateGameResponse, CreatePlayerResponse, GetGameStateResponse, MoveResponse, GetStatisticsResponse,
};
use crate::player::Player;

use arraydeque::{ArrayDeque, Wrapping};
use serde_json::Value;
use std::{cell::RefCell, collections::HashMap, rc::Rc, rc::Weak, ops::AddAssign};
use crate::settings::{PLAYERS_MAX_COUNT, GAMES_MAX_COUNT};

pub struct GameStatistics {
    // overall players count that has been registered
    pub players_created: u64,
    // overall players count that has been created
    pub games_created: u64,
    // overall move count that has been made
    pub moves_count: i64,
}

pub struct GameManager {
    players: ArrayDeque<[Rc<RefCell<Player>>; PLAYERS_MAX_COUNT], Wrapping>,
    games: ArrayDeque<[Rc<RefCell<Game>>; GAMES_MAX_COUNT], Wrapping>,
    // TODO: String key should be replaced with Cow<'a, str>. After that signatures of all public
    // functions also should be changed similar to https://jwilm.io/blog/from-str-to-cow/.
    players_by_name: HashMap<String, Weak<RefCell<Player>>>,
    game_statistics: RefCell<GameStatistics>,
}

impl GameManager {
    pub fn new() -> Self {
        GameManager {
            players: ArrayDeque::new(),
            games: ArrayDeque::new(),
            players_by_name: HashMap::new(),
            game_statistics: RefCell::new(GameStatistics {
                players_created: 0,
                games_created: 0,
                moves_count: 0,
            }),
        }
    }

    /// Marks an empty position on the board by user's tile type. Returns MoveResponse structure
    /// as a serde_json Value.
    pub fn make_move(&self,
        player_name: String,
        coords: (usize, usize),
    ) -> AppResult<Value> {
        let game = self.get_player_game(&player_name)?;
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

        self.game_statistics.borrow_mut().moves_count.add_assign(1);

        serde_json::to_value(response).map_err(Into::into)
    }

    /// Creates a new player with given player name.
    pub fn create_player(&mut self, player_name: String) -> AppResult<Value> {
        let new_player = Rc::new(RefCell::new(Player::new(player_name.clone())));
        if let Some(_) = self.players_by_name.get(&player_name) {
           return Err("User with this name is already registered, please choose another one".to_owned()).map_err(Into::into);
        }

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

        self.game_statistics.borrow_mut().players_created.add_assign(1);

        serde_json::to_value(response).map_err(Into::into)
    }

    /// Creates a new game for provided player. Note that the previous one is deleted (if it
    /// present) and won't be accessed anymore. Returns CreateGameResponse as a serde_json Value if
    /// 'X' tile type has been chosen and MoveResponse otherwise.
    pub fn create_game(
        &mut self,
        player_name: String,
        player_tile: Tile,
    ) -> AppResult<Value> {
        let player = self.get_player(&player_name)?;

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

        self.game_statistics.borrow_mut().games_created.add_assign(1);
        self.games.push_back(game_state);

        response.map_err(Into::into)
    }

    /// Returns current game state for provided user as a GetGameStateResponse serde_json Value.
    pub fn get_game_state(&self, player_name: String) -> AppResult<Value> {
        let game = self.get_player_game(&player_name)?;
        let (chosen_tile, board) = game.borrow().get_state();

        let response = GetGameStateResponse {
            player_name,
            player_tile: chosen_tile.to_char(),
            board,
        };
        serde_json::to_value(response).map_err(Into::into)
    }

    /// Returns statistics of application usage.
    pub fn get_statistics(&self) -> AppResult<Value> {
        let response = GetStatisticsResponse {
            players_created: self.game_statistics.borrow().players_created,
            games_created: self.game_statistics.borrow().games_created,
            moves_count: self.game_statistics.borrow().moves_count,
        };
        serde_json::to_value(response).map_err(Into::into)
    }

    fn get_player(&self, player_name: &str) -> AppResult<Rc<RefCell<Player>>> {
        // try to find player by name in players_by_name and then convert Weak<Player> to Rc<Player>
        match self.players_by_name.get(&player_name.to_owned()) {
            Some(player) => player.upgrade().ok_or_else(|| {
                "Internal error occurred - player has been already removed".to_owned()
            }),
            None => Err(format!(
                "Player with name {} wasn't found",
                player_name
            )),
        }.map_err(Into::into)
    }

    fn get_player_game(
        &self,
        player_name: &str,
    ) -> AppResult<Rc<RefCell<Game>>> {
        self
            // returns Rc<RefCell<Player>> if success
            .get_player(player_name)?
            // borrows a mutable link to Player from RefCell
            .borrow_mut()
            // gets Weak<RefCell<Game>> from Player
            .game
            // tries to upgrade Weak<RefCell<Game>> to Rc<RefCell<Game>>
            .upgrade()
            .ok_or_else(|| "Sorry! Your game was deleted, but you can start a new one".to_owned())
            .map_err(Into::into)
    }
}
