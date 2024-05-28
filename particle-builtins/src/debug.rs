/*
 * Copyright 2024 Fluence DAO
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

use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::builtins::CustomService;

pub fn fmt_custom_services(
    services: &RwLock<HashMap<String, CustomService>>,
    fmt: &mut std::fmt::Formatter<'_>,
) -> Result<(), std::fmt::Error> {
    fmt.debug_map()
        .entries(
            services
                .blocking_read()
                .iter()
                .map(|(sid, fs)| (sid, fs.functions.keys().collect::<Vec<_>>())),
        )
        .finish()
}
