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

#[macro_use]
extern crate fstrings;

mod chat {
    mod chat;
}

use serde::{Deserialize, Serialize};
use serde_json;

#[test]
fn deserialize() {
    let msg = FooMessage::Particle(<_>::default());
    let bytes = serde_json::to_vec(&msg).unwrap();
    let test_msg: Result<FooMessage, _> = serde_json::from_slice(&bytes);
    println!("{:?}", test_msg);
    test_msg.unwrap();
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct FooParticle {
    pub id: String,
    pub timestamp: u64,
    pub ttl: u32,
    pub script: String,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "action")]
pub enum FooMessage {
    Particle(FooParticle),
}
