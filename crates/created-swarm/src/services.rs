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

use crate::CreatedSwarm;
use fluence_libp2p::PeerId;
use futures::future::BoxFuture;
use futures::FutureExt;
use maplit::hashmap;
use particle_execution::FunctionOutcome;
use serde_json::json;

pub async fn add_print<'a>(swarms: impl Iterator<Item = &'a mut CreatedSwarm>) {
    let print = |peer_id: PeerId| -> Box<
        dyn Fn(_, _) -> BoxFuture<'static, FunctionOutcome> + 'static + Send + Sync,
    > {
        Box::new(move |args: particle_args::Args, _| {
            async move {
                println!("{} printing {}", peer_id, json!(args.function_args));
                FunctionOutcome::Empty
            }
            .boxed()
        })
    };
    for s in swarms {
        s.aquamarine_api
            .clone()
            .add_service(
                "test".into(),
                hashmap! {
                    "print".to_string() => print(s.peer_id).into(),
                },
            )
            .await
            .expect("add service");
    }
}
