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

use crate::game::{Game, Tile};
use crate::player::Player;
use arraydeque::{ArrayDeque, Wrapping};
use log::info;
use serde_json::json;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::result::Result;

mod settings {
    pub const PLAYERS_MAX_COUNT: usize = 1024;
    pub const GAMES_MAX_COUNT: usize = 1024;
}

pub struct GameManager {
    players: ArrayDeque<[Rc<RefCell<Player>>; settings::PLAYERS_MAX_COUNT], Wrapping>,
    games: ArrayDeque<[Rc<RefCell<Game>>; settings::GAMES_MAX_COUNT], Wrapping>,
    names_to_players: HashMap<String, Weak<RefCell<Player>>>,
}

impl GameManager {
    pub fn new() -> Self {
        GameManager {
            players: ArrayDeque::new(),
            games: ArrayDeque::new(),
            names_to_players: HashMap::new(),
        }
    }

    pub fn make_move(
        &self,
        player_name: String,
        player_sign: String,
        coords: (i32, i32),
    ) -> String {
        let player = match self.get_player(player_name, player_sign) {
            Ok(player) => player,
            Err(err) => return json!({ "result": err }).to_string(),
        };

        let game = player.borrow_mut().game.upgrade().unwrap();
        let result = game.borrow_mut().player_move(coords);

        json!({
        "result": "moved successfully"
        })
        .to_string()
    }

    pub fn create_player(&mut self, player_name: String, player_sign: String) -> String {
        let new_player = Rc::new(RefCell::new(Player::new(player_name, player_sign)));

        self.names_to_players
            .insert(new_player.borrow().name.clone(), Rc::downgrade(&new_player));

        if let Some(prev) = self.players.push_back(new_player) {
            // if some elements poped from of deque, delete a link for it from names_to_players
            self.names_to_players.remove(&prev.borrow().name);
        }

        json!({
        "result": "new player has been successfully created"
        })
        .to_string()
    }

    pub fn create_game(
        &mut self,
        player_name: String,
        player_sign: String,
        player_tile: Tile,
    ) -> String {
        let player = match self.get_player(player_name, player_sign) {
            Ok(player) => player,
            Err(err) => return json!({ "result": err }).to_string(),
        };

        let game_state = Rc::new(RefCell::new(Game::new(player_tile)));
        player.borrow_mut().game = Rc::downgrade(&game_state);
        self.games.push_back(game_state);

        json!({
            "result": "new game has been successfully created"
        })
        .to_string()
    }

    pub fn get_game_state(&self, player_name: String, player_sign: String) -> String {
        json!({
        "result": "game state is"
        })
        .to_string()
    }

    fn get_player(&self, name: String, sign: String) -> Result<Rc<RefCell<Player>>, String> {
        info!(
            "get_player {} {}",
            self.names_to_players.len(),
            self.players.len()
        );

        for (name, _) in &self.names_to_players {
            info!("self.names_to_players content is {}", name);
        }

        // try to find player by name in names_to_players and then convert Weak<Player> to Rc<Player>
        let player = match self.names_to_players.get(&name) {
            Some(player) => player.upgrade().ok_or_else(|| {
                "Internal error occurred - player has been already removed".to_owned()
            }),
            None => Err(format!(
                "Player with name {} and sign {} wasn't found",
                name, sign
            )),
        }?;

        // checks player signature
        if player.borrow().sign == sign {
            return Ok(player);
        }

        // errors are not distinguishable to not to give out the names of players
        Err(format!(
            "Player with name {} and sign {} wasn't found",
            name, sign
        ))
    }
}
