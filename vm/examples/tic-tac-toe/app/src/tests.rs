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
#![warn(dead_code)]

use crate::main;
use serde_json::json;

#[test]
fn integration_sql_test() {
    let create_player_1 = create_player_json("John".to_owned(), "so_secret_key".to_owned(), 'X');
    let create_player_2 = create_player_json("John2".to_owned(), "so_secret_key2".to_owned(), 'O');

    let create_game_1 = create_game_json("John".to_owned(), "so_secret_key".to_owned());
    let create_game_2 = create_game_json("John2".to_owned(), "so_secret_key2".to_owned());

    let player_move_1 = create_move_json("John2".to_owned(), "so_secret_key2".to_owned(), 0, 0);
    let player_move_2 = create_move_json("John2".to_owned(), "so_secret_key2".to_owned(), 0, 1);
    let player_move_3 = create_move_json("John2".to_owned(), "so_secret_key2".to_owned(), 0, 2);

   let tt = execute_request(create_player_1.as_str());

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

use std::mem;
use std::ptr;

pub unsafe fn read_result_from_mem(ptr: *mut u8) -> Vec<u8> {
    const RESULT_SIZE_BYTES: usize = 4;

    // read string length from current pointer
    let mut len_as_bytes: [u8; RESULT_SIZE_BYTES] = [0; RESULT_SIZE_BYTES];
    ptr::copy_nonoverlapping(ptr, len_as_bytes.as_mut_ptr(), RESULT_SIZE_BYTES);

    let input_len: u32 = mem::transmute(len_as_bytes);
    let total_len = input_len as usize + RESULT_SIZE_BYTES;

    // creates object from raw bytes
    let mut input = Vec::from_raw_parts(ptr, total_len, total_len);

    // drains RESULT_SIZE_BYTES from the beginning of created vector, it allows to effectively
    // skips (without additional allocations) length of the result.
    {
        input.drain(0..4);
    }
    input
}

/// Executes sql and returns result as a String.
fn execute_request(sql: &str) -> String {
    unsafe {
        use std::mem;

        let mut sql_str = sql.to_string();
        // executes query
        let result_str_ptr = super::invoke(sql_str.as_bytes_mut().as_mut_ptr(), sql_str.len());
        // ownership of memory of 'sql_str' will be taken through 'ptr' inside
        // 'invoke' method, have to forget 'sql_str'  for prevent deallocation
        mem::forget(sql_str);
        // converts results
        let result_str = read_result_from_mem(result_str_ptr.as_ptr());
        String::from_utf8(result_str).unwrap()
    }
}
