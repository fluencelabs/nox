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

/*
  join -> {id}
  bet {id, money, result} -> {outcome, player_balance}
  get_account_state {id} -> {account_state}
*/
#[derive(Serialize, Deserialize)]
pub struct Request {
    pub action: String,
}

#[derive(Serialize, Deserialize)]
pub struct BetRequest {
    pub player_id: u64,
    pub placement: u8,
    pub bet_amount: u32,
}

#[derive(Serialize, Deserialize)]
pub struct GetBalanceRequest {
    pub player_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct JoinResponse {
    pub player_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct BetResponse {
    pub outcome: u8,
    pub player_balance: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GetBalanceResponse {
    pub player_balance: u64,
}
