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

use crate::utils::ConnectedClient;
use parity_multiaddr::Multiaddr;

static TEST_MODULE: &[u8] = include_bytes!("../artifacts/test_module_wit.wasi.wasm");

#[derive(serde::Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct Function {
    pub name: String,
    pub input_types: Vec<String>,
    pub output_types: Vec<String>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Module {
    pub name: String,
    pub functions: Vec<Function>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Interface {
    pub modules: Vec<Module>,
}

impl PartialEq for Interface {
    fn eq(&self, right: &Self) -> bool {
        for left in self.modules.iter() {
            let right = right.modules.iter().find(|m| m.name == left.name).unwrap();
            if left.functions.len() != right.functions.len() {
                return false;
            }
            for left in left.functions.iter() {
                let right = right
                    .functions
                    .iter()
                    .find(|f| f.name == left.name)
                    .unwrap();
                if right != left {
                    return false;
                }
            }
        }

        true
    }
}

pub fn test_module() -> Vec<u8> {
    TEST_MODULE.to_vec()
}

pub fn add_test_modules(swarm: Multiaddr) {
    let mut client = ConnectedClient::connect_to(swarm).expect("connect client");
    #[rustfmt::skip]
    let config = |name| toml::from_str(format!(
    r#"
        name = "{}"
        mem_pages_count = 100
        logger_enabled = true
        [module.wasi]
        envs = []
        preopened_files = ["./tests/artifacts"]
    "#, name).as_str()).expect("parse config");

    client.add_module(test_module().as_slice(), config("test_one"));
    client.add_module(test_module().as_slice(), config("test_two"));
}
