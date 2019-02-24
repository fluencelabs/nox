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

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub action: String,
    pub player_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerTile {
    pub tile: char,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerMove {
    pub coords: (usize, usize),
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub player_name: String,
    pub error: String,
}

#[derive(Serialize, Deserialize)]
pub struct MoveResponse {
    pub player_name: String,
    pub winner: String,
    pub coords: (usize, usize),
}

#[derive(Serialize, Deserialize)]
pub struct CreatePlayerResponse {
    pub player_name: String,
    pub result: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateGameResponse {
    pub player_name: String,
    pub result: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetGameStateResponse {
    pub player_name: String,
    pub player_tile: char,
    pub board: Vec<char>,
}

#[derive(Serialize, Deserialize)]
pub struct GetStatisticsResponse {
    pub players_created: u64,
    pub games_created: u64,
    pub moves_count: i64,
}
