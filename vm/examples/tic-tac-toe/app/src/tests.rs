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

// TODO: add more tests

#[test]
fn x_tile_win() {
    let create_player = create_player_json("John".to_owned(), "so_secret_key".to_owned());
    assert_eq!(
        main(create_player),
        r#"{"player_name":"John","result":"A new player has been successfully created"}"#
    );

    let create_game = create_game_json("John".to_owned(), "so_secret_key".to_owned(), 'X');
    assert_eq!(
        main(create_game),
        r#"{"player_name":"John","result":"A new game has been successfully created"}"#
    );

    let player_move = create_move_json("John".to_owned(), "so_secret_key".to_owned(), 0, 0);
    assert_eq!(
        main(player_move),
        r#"{"coords":[0,1],"player_name":"John","winner":"None"}"#
    );

    let player_move = create_move_json("John".to_owned(), "so_secret_key".to_owned(), 1, 0);
    assert_eq!(
        main(player_move),
        r#"{"coords":[0,2],"player_name":"John","winner":"None"}"#
    );

    let player_move = create_move_json("John".to_owned(), "so_secret_key".to_owned(), 2, 0);
    assert_eq!(main(player_move), r#"{"coords":[18446744073709551615,18446744073709551615],"player_name":"John","winner":"X"}"#);

    let get_state = get_state_json("John".to_owned(), "so_secret_key".to_owned());
    assert_eq!(
        main(get_state),
        r#"{"board":["X","O","O","X","_","_","X","_","_"],"player_name":"John","player_tile":"X"}"#
    );
}

#[test]
fn o_tile_win() {
    let create_player = create_player_json("John2".to_owned(), "so_secret_key2".to_owned());
    assert_eq!(
        main(create_player),
        r#"{"player_name":"John2","result":"A new player has been successfully created"}"#
    );

    let create_game = create_game_json("John2".to_owned(), "so_secret_key2".to_owned(), 'O');
    assert_eq!(
        main(create_game),
        r#"{"coords":[0,0],"player_name":"John2","winner":"None"}"#
    );

    let player_move = create_move_json("John2".to_owned(), "so_secret_key2".to_owned(), 0, 2);
    assert_eq!(
        main(player_move),
        r#"{"coords":[0,1],"player_name":"John2","winner":"None"}"#
    );

    let player_move = create_move_json("John2".to_owned(), "so_secret_key2".to_owned(), 1, 2);
    assert_eq!(
        main(player_move),
        r#"{"coords":[1,0],"player_name":"John2","winner":"None"}"#
    );

    let player_move = create_move_json("John2".to_owned(), "so_secret_key2".to_owned(), 2, 2);
    assert_eq!(
        main(player_move),
        r#"{"coords":[1,1],"player_name":"John2","winner":"None"}"#
    );

    let get_state = get_state_json("John2".to_owned(), "so_secret_key2".to_owned());
    assert_eq!(main(get_state), r#"{"board":["X","X","O","X","X","O","_","_","O"],"player_name":"John2","player_tile":"O"}"#);
}

#[test]
fn app_win() {
    let create_player = create_player_json("John3".to_owned(), "so_secret_key3".to_owned());
    assert_eq!(
        main(create_player),
        r#"{"player_name":"John3","result":"A new player has been successfully created"}"#
    );

    let create_game = create_game_json("John3".to_owned(), "so_secret_key3".to_owned(), 'O');
    assert_eq!(
        main(create_game),
        r#"{"coords":[0,0],"player_name":"John3","winner":"None"}"#
    );

    let player_move = create_move_json("John3".to_owned(), "so_secret_key3".to_owned(), 2, 0);
    assert_eq!(
        main(player_move),
        r#"{"coords":[0,1],"player_name":"John3","winner":"None"}"#
    );

    let player_move = create_move_json("John3".to_owned(), "so_secret_key3".to_owned(), 2, 1);
    assert_eq!(
        main(player_move),
        r#"{"coords":[0,2],"player_name":"John3","winner":"X"}"#
    );

    let player_move = create_move_json("John3".to_owned(), "so_secret_key3".to_owned(), 2, 2);
    assert_eq!(
        main(player_move),
        r#"{"error":"Player X has already won this game"}"#
    );

    let get_state = get_state_json("John3".to_owned(), "so_secret_key3".to_owned());
    assert_eq!(main(get_state), r#"{"board":["X","X","X","_","_","_","O","O","_"],"player_name":"John3","player_tile":"O"}"#);
}

fn create_move_json(player_name: String, player_sign: String, x: usize, y: usize) -> String {
    generate_json(
        "move".to_owned(),
        player_name,
        player_sign,
        format!(r#", "coords": [{}, {}]"#, x, y),
    )
}

fn create_player_json(player_name: String, player_sign: String) -> String {
    generate_json(
        "create_player".to_owned(),
        player_name,
        player_sign,
        "".to_owned(),
    )
}

fn create_game_json(player_name: String, player_sign: String, tile: char) -> String {
    generate_json(
        "create_game".to_owned(),
        player_name,
        player_sign,
        format!(r#", "tile": "{}""#, tile),
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
    // TODO: move to json! macro
    format!(
        r#"{{ "action": "{}", "player_name": "{}", "player_sign": "{}" {} }}"#,
        action, player_name, player_sign, additional_fields
    )
}
