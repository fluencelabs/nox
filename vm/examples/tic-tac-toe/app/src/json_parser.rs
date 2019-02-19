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
use serde_json::Serializer;

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub action: String,
    pub player_name: String,
    pub player_sign: String,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerTile {
    pub tile: char,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerMove {
    pub coords: (i32, i32),
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    error_code: i32,
    player_name: String,
    player_sign: String,
    params: serde_json::Value,
}
