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

use crate::utils::{
    create_memory_maddr, create_swarm, make_swarms_with, CreatedSwarm, SwarmConfig, KAD_TIMEOUT,
};
use fluence_faas_service::RawModulesConfig;
use parity_multiaddr::Multiaddr;
use std::thread::sleep;

static TEST_MODULE: &[u8] = include_bytes!("../artifacts/test_module_wit.wasi.wasm");
static WASM_CONFIG: &str = r#"
modules_dir = ""

[[core_module]]
    name = "test_one.wasm"
    mem_pages_count = 100
    logger_enabled = true        
[core_module.wasi]
    envs = []
    preopened_files = ["./tests/artifacts"]
    mapped_dirs = { "tmp" = "./tests/artifacts" }
    
[[core_module]]
    name = "test_two.wasm"
    mem_pages_count = 100
    logger_enabled = true
[core_module.wasi]
    envs = []
    preopened_files = ["./tests/artifacts"]
    mapped_dirs = { "tmp" = "./tests/artifacts" }

[rpc_module]
    mem_pages_count = 100
    logger_enabled = true

    [rpc_module.wasi]
    envs = []
    preopened_files = ["./tests/artifacts"]
    mapped_dirs = { "tmp" = "./tests/artifacts" }
"#;

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

pub fn faas_config(bs: Vec<Multiaddr>, maddr: Multiaddr) -> SwarmConfig<'static> {
    let wasm_config: RawModulesConfig =
        toml::from_str(WASM_CONFIG).expect("parse module config");

    let wasm_modules = vec![
        ("test_one.wasm".to_string(), test_module()),
        ("test_two.wasm".to_string(), test_module()),
    ];

    let mut config = SwarmConfig::new(bs, maddr);
    config.wasm_modules = wasm_modules;
    config.wasm_config = wasm_config;
    config
}

pub fn test_module() -> Vec<u8> {
    TEST_MODULE.to_vec()
}

pub fn start_faas() -> CreatedSwarm {
    let swarms = make_swarms_with(
        1,
        |bs, maddr| create_swarm(faas_config(bs, maddr)),
        create_memory_maddr,
        true,
    );
    sleep(KAD_TIMEOUT);

    swarms.into_iter().next().unwrap()
}
