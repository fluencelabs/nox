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

use futures::FutureExt;
use particle_builtins::{ok, CustomService, NodeInfo};
use particle_execution::ServiceFunction;
use serde_json::json;

pub fn make_peer_builtin(node_info: NodeInfo) -> (String, CustomService) {
    (
        "peer".to_string(),
        CustomService::new(
            vec![("identify", make_peer_identify_closure(node_info))],
            None,
        ),
    )
}
fn make_peer_identify_closure(node_info: NodeInfo) -> ServiceFunction {
    ServiceFunction::Immut(Box::new(move |_args, _params| {
        let node_info = node_info.clone();
        async move { ok(json!(node_info)) }.boxed()
    }))
}
