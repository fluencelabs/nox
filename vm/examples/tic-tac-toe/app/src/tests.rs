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
use crate::main;
use serde_json::json;

#[test]
fn x_tile_win() {
    let create_player = create_player_json("John".to_owned(), "so_secret_key".to_owned(), 'X');
    main(create_player);

    let create_game = create_game_json("John".to_owned(), "so_secret_key".to_owned());
    main(create_game);

    let player_move_1 = create_move_json("John".to_owned(), "so_secret_key".to_owned(), 0, 0);
    main(player_move_1);

    let player_move_2 = create_move_json("John".to_owned(), "so_secret_key".to_owned(), 1, 0);
    main(player_move_2);

    let player_move_3 = create_move_json("John".to_owned(), "so_secret_key".to_owned(), 2, 0);
    main(player_move_3);
}

#[test]
fn o_tile_win() {
    let create_player = create_player_json("John".to_owned(), "so_secret_key2".to_owned(), 'O');
    main(create_player);

    let create_game = create_game_json("John2".to_owned(), "so_secret_key2".to_owned());
    main(create_game);

    let player_move_1 = create_move_json("John2".to_owned(), "so_secret_key2".to_owned(), 0, 2);
    main(player_move_1);

    let player_move_2 = create_move_json("John2".to_owned(), "so_secret_key2".to_owned(), 1, 2);
    main(player_move_2);

    let player_move_3 = create_move_json("John2".to_owned(), "so_secret_key2".to_owned(), 2, 2);
    main(player_move_3);
}

fn create_move_json(player_name: String, player_sign: String, x: usize, y: usize) -> String {
    generate_json(
        "move".to_owned(),
        player_name,
        player_sign,
        format!(", \"coords\": [{}, {}]", x, y),
    )
}

fn create_player_json(player_name: String, player_sign: String, tile: char) -> String {
    generate_json(
        "create_player".to_owned(),
        player_name,
        player_sign,
        format!(", \"tile\": \"{}\"", tile),
    )
}

fn create_game_json(player_name: String, player_sign: String) -> String {
    generate_json(
        "create_game".to_owned(),
        player_name,
        player_sign,
        "".to_owned(),
    )
}

fn get_state_json(player_name: String, player_sign: String) -> String {
    generate_json(
        "get_game_state".to_owned(),
        player_name,
        player_sign,
        "".to_owned(),
    )
}

fn generate_json(
    action: String,
    player_name: String,
    player_sign: String,
    additional_fields: String,
) -> String {
    let result_json = json!({
       "action": action,
       "player_name": player_name,
       "player_sign": player_sign
    });

    result_json.to_string()
}
