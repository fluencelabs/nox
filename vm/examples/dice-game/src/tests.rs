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
use crate::request_response::{Request, Response};
use crate::main;

// TODO: add more tests

#[test]
fn correct_bets() {
    let response = Response::Join { player_id: 0 };
    assert_eq!(
        main(create_join_request()),
        serde_json::to_string(&response).unwrap()
    );

    let response = Response::Join { player_id: 1 };
    assert_eq!(
        main(create_join_request()),
        serde_json::to_string(&response).unwrap()
    );

    let response = Response::Bet {
        outcome: 6,
        player_balance: 85,
    };
    assert_eq!(
        main(create_bet_json(0, 1, 15)),
        serde_json::to_string(&response).unwrap()
    );

    let response = Response::Bet {
        outcome: 4,
        player_balance: 85,
    };
    assert_eq!(
        main(create_bet_json(1, 1, 15)),
        serde_json::to_string(&response).unwrap()
    );

    let response = Response::Bet {
        outcome: 6,
        player_balance: 510,
    };
    assert_eq!(
        main(create_bet_json(0, 6, 85)),
        serde_json::to_string(&response).unwrap()
    );

    let response = Response::Bet {
        outcome: 2,
        player_balance: 0,
    };
    assert_eq!(
        main(create_bet_json(1, 1, 85)),
        serde_json::to_string(&response).unwrap()
    );

    let response = Response::GetBalance {
        player_balance: 510,
    };
    assert_eq!(
        main(create_get_balance_json(0)),
        serde_json::to_string(&response).unwrap()
    );
}

#[test]
fn incorrect_bets() {
    let response = Response::Join { player_id: 0 };
    assert_eq!(
        main(create_join_request()),
        serde_json::to_string(&response).unwrap()
    );

    let response = Response::Error {
        message: "Incorrect placement, please choose number from 1 to 6".to_string(),
    };
    assert_eq!(
        main(create_bet_json(0, 7, 15)),
        serde_json::to_string(&response).unwrap()
    );

    let response = Response::Error {
        message: "Player with id 1 wasn\'t found".to_string(),
    };
    assert_eq!(
        main(create_bet_json(1, 1, 0)),
        serde_json::to_string(&response).unwrap()
    );

    let response = Response::Error {
        message: "Player hasn\'t enough money: player\'s current balance is 100 while the bet is 4294967295".to_string()
    };
    assert_eq!(
        main(create_bet_json(0, 6, std::u32::MAX)),
        serde_json::to_string(&response).unwrap()
    );
}

fn create_join_request() -> String {
    let request = Request::Join;
    serde_json::to_value(request).unwrap().to_string()
}

fn create_bet_json(player_id: u64, placement: u8, bet_amount: u32) -> String {
    let request = Request::Bet {
        player_id,
        placement,
        bet_amount,
    };
    serde_json::to_value(request).unwrap().to_string()
}

fn create_get_balance_json(player_id: u64) -> String {
    let request = Request::GetBalance { player_id };
    serde_json::to_value(request).unwrap().to_string()
}
