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

use boolinator::Boolinator;
use serde_json::json;
use std::convert::From;
use std::fmt;
use std::result::Result;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Tile {
    X,
    O,
}

impl Tile {
    pub fn from_char(ch: char) -> Option<Self> {
        match ch {
            'X' => Some(Tile::X),
            'O' => Some(Tile::O),
            _ => None,
        }
    }

    // returns tile type of opposite player
    pub fn other(self) -> Self {
        match self {
            Tile::X => Tile::O,
            Tile::O => Tile::X,
        }
    }
}

impl fmt::Display for Tile {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let tile_as_str = match self {
            Tile::X => "X",
            Tile::O => "O",
        };
        fmt.write_str(tile_as_str)?;
        Ok(())
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Winner {
    X,
    O,
    Draw,
}

impl fmt::Display for Winner {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let winner_as_str = match self {
            Winner::X => "X",
            Winner::O => "O",
            Winner::Draw => "Draw",
        };
        fmt.write_str(winner_as_str)?;
        Ok(())
    }
}

impl From<Tile> for Winner {
    fn from(tile: Tile) -> Self {
        match tile {
            Tile::X => Winner::X,
            Tile::O => Winner::O,
        }
    }
}

pub struct Game {
    board: [[Option<Tile>; 3]; 3],
    chosen_tile: Tile,
}

impl Game {
    pub fn new(player_tile: Tile) -> Self {
        Game {
            board: [[None; 3]; 3],
            chosen_tile: player_tile,
        }
    }

    pub fn get_winner(&self) -> Option<Winner> {
        // check columns
        for col in 0..2 {
            if self.board[0][col].is_some()
                && (self.board[0][col] == self.board[1][col])
                && (self.board[1][col] == self.board[2][col])
            {
                return self.board[0][col].map(|tile| tile.into());
            }
        }

        // check rows
        for row in 0..2 {
            if self.board[row][0].is_some()
                && (self.board[row][0] == self.board[row][0])
                && (self.board[row][1] == self.board[row][2])
            {
                return self.board[row][0].map(|tile| tile.into());
            }
        }

        // check the left-right diagonal
        if self.board[0][0].is_some()
            && (self.board[0][0] == self.board[1][1])
            && (self.board[1][1] == self.board[2][2])
        {
            return self.board[0][0].map(|tile| tile.into());
        }

        // check the right-left diagonal
        if self.board[2][0].is_some()
            && (self.board[2][0] == self.board[1][1])
            && (self.board[1][1] == self.board[0][2])
        {
            return self.board[2][0].map(|tile| tile.into());
        }

        // check that all tiles are not empty (a draw condition)
        self.board
            .iter()
            .all(|row| row.iter().all(|cell| cell.is_some()))
            .as_some(Winner::Draw)
    }

    pub fn player_move(&mut self, coord: (i32, i32)) -> Result<Option<Winner>, String> {
        if let Some(player) = self.get_winner() {
            return Ok(Some(player));
        }

        if coord.0 < 0 || coord.0 > 2 || coord.1 < 0 || coord.1 > 2 {
            return Err("Invalid coordinates".to_owned());
        }

        self.board[coord.0 as usize][coord.1 as usize]
            .is_none()
            .ok_or_else(|| "Please choose a free position".to_owned())?;

        self.board[coord.0 as usize][coord.1 as usize] = Some(self.chosen_tile);
        self.app_move();

        Ok(self.get_winner())
    }

    pub fn get_state(&self) -> String {
        json!({
        "chosen_tile": format!("{}", self.chosen_tile),
        "board": ""
        })
        .to_string()
    }

    fn app_move(&mut self) {
        let mut possible_actions = Vec::new();

        for tile in self.board.iter().flat_map(|r| r.iter()) {
            if let None = tile {
                possible_actions.push(tile);
            }
        }

        if possible_actions.is_empty() {
            return;
        }

        // TODO: use more complicated strategy
        possible_actions[0].map(|f| self.chosen_tile.other());
    }
}

impl fmt::Display for Game {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut board_str = String::new();

        for tile in self.board.iter().flat_map(|r| r.iter()) {
            match tile {
                Some(tile) => board_str.push_str((tile.to_string() + ", ").as_str()),
                None => board_str.push_str("_, "),
            }
        }

        let str = json!({
            "chosen_tile": format!("\"{}\"", self.chosen_tile),
            "board": board_str
        })
        .to_string();

        fmt.write_str(str.as_str())?;
        Ok(())
    }
}
