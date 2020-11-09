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

#![allow(dead_code)]

use aquamarine_vm::AquamarineVM;
use particle_protocol::Particle;
use serde_json::Value as JValue;
use std::cell::RefCell;
use std::collections::HashMap;

struct SingletonVM {
    vm: AquamarineVM,
    load_map: HashMap<String, JValue>,
}

impl SingletonVM {
    pub fn new() -> Self {
        unimplemented!()
    }

    pub fn make_particle(&mut self) -> Particle {
        unimplemented!()
    }

    pub fn read_args(&mut self, _particle: Particle) -> JValue {
        unimplemented!()
    }
}

thread_local! {
    static VM: RefCell<SingletonVM> = RefCell::new(SingletonVM::new());
}
