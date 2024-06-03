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

use prometheus_client::metrics::info::Info;
use prometheus_client::registry::Registry;

pub fn add_info_metrics(
    registry: &mut Registry,
    node_version: String,
    air_version: String,
    spell_version: String,
) {
    let sub_registry = registry.sub_registry_with_prefix("nox");
    let info = Info::new(vec![
        ("node_version", node_version),
        ("air_version", air_version),
        ("spell_version", spell_version),
    ]);
    sub_registry.register("build", "Nox Info", info);
}
