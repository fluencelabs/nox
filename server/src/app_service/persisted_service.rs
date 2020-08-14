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

use fluence_app_service::RawModulesConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersistedService<'a> {
    #[serde(borrow)]
    pub service_id: &'a str,
    pub config: RawModulesConfig,
}

impl<'a> PersistedService<'a> {
    pub fn new(service_id: &'a str, config: RawModulesConfig) -> Self {
        Self { service_id, config }
    }
}
