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

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blueprint {
    pub name: String,
    #[serde(default = "uuid")]
    pub id: String,
    pub dependencies: Vec<String>,
}

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl Blueprint {
    pub fn new(name: String, id: String, dependencies: Vec<String>) -> Self {
        Self {
            name,
            id,
            dependencies,
        }
    }
}
