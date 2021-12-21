/*
 * Copyright 2021 Fluence Labs Limited
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

pub const SERVICES: &'static str = "./services";

mod src {
    mod aqua_dht {
        mod aqua_dht;
        mod pubsub;

        #[derive(serde::Deserialize, Debug)]
        #[allow(dead_code)] // allowing dead_code here because the fields are never read, only written
        pub struct Record {
            value: String,
            peer_id: String,
            set_by: String,
            relay_id: Vec<String>,
            service_id: Vec<String>,
            timestamp_created: u64,
            weight: u32,
        }
    }

    mod builtins_deployer;
}

pub fn load_script(name: &str) -> String {
    std::fs::read_to_string(format!("./tests/src/aqua_dht/aqua/{}", name)).unwrap()
}
