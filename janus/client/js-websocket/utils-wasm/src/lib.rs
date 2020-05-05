/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
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
