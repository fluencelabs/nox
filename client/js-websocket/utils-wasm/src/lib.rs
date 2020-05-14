/*
 * Copyright 2020 Fluence Labs Limited
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

use faster_hex::{hex_decode, hex_string};
use sha3::digest::Digest;
use sha3::{Keccak256, Sha3_256};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn encode(data: &[u8]) -> String {
    hex_string(data).unwrap()
}

#[wasm_bindgen]
pub fn decode(str: &str) -> Vec<u8> {
    let bytes = str.as_bytes();
    let mut dst = Vec::with_capacity(bytes.len());
    hex_decode(bytes, &mut dst);
    dst
}

#[wasm_bindgen]
pub fn sha3_256(input: &[u8]) -> Vec<u8> {
    hash_vec::<Sha3_256>(input)
}

#[wasm_bindgen]
pub fn keccak_256(input: &[u8]) -> Vec<u8> {
    hash_vec::<Keccak256>(input)
}

fn hash_vec<D: Digest + Default>(input: &[u8]) -> Vec<u8> {
    let mut sh = D::default();
    sh.input(input);
    sh.result().to_vec()
}
