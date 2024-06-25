/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
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
