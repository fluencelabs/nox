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
