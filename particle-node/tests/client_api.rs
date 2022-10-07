/*
 * Copyright 2021 Fluence Labs Limited
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

use std::time::Duration;

use async_std::task::block_on;
use eyre::WrapErr;
use futures::channel::oneshot::channel;
use futures::future::BoxFuture;
use futures::FutureExt;
use maplit::hashmap;
use serde_json::json;
use serde_json::Value as JValue;

use connected_client::ConnectedClient;
use created_swarm::{make_swarms, CreatedSwarm};
use now_millis::now_ms;
use particle_execution::FunctionOutcome;
use particle_protocol::Particle;
use test_constants::PARTICLE_TTL;
use test_utils::timeout;
use uuid_utils::uuid;

#[test]
fn call_custom_service() {
    let swarms = make_swarms(2);

    // pub type Output<'a> = BoxFuture<'a, FunctionOutcome>;
    //
    // pub type ServiceFunction =
    // Box<dyn FnMut(Args, ParticleParams) -> Output<'static> + 'static + Send + Sync>;

    let closure: Box<
        dyn FnMut(_, _) -> BoxFuture<'static, FunctionOutcome> + 'static + Send + Sync,
    > = Box::new(move |args, params| {
        async move {
            println!("got call!!! {:?} {:?}", args, params);
            FunctionOutcome::Ok(json!("hello!"))
        }
        .boxed()
    });

    let add_first_f = swarms[0]
        .aquamarine_api
        .clone()
        .add_service("hello".into(), hashmap! { "world".to_string() => closure });

    let (outlet, inlet) = channel();
    let mut outlet = Some(outlet);
    let closure: Box<
        dyn FnMut(_, _) -> BoxFuture<'static, FunctionOutcome> + 'static + Send + Sync,
    > = Box::new(move |args, params| {
        let mut outlet = outlet.take();
        async move {
            let outlet = outlet.take();
            println!("got return call!!! {:?} {:?}", args, params);
            outlet.map(|out| out.send((args, params)));
            FunctionOutcome::Empty
        }
        .boxed()
    });

    let add_second_f = swarms[1]
        .aquamarine_api
        .clone()
        .add_service("op".into(), hashmap! { "return".to_string() => closure });

    let script = format!(
        r#"
        (seq
            (call {} ("hello" "world") [] result)
            (call %init_peer_id% ("op" "return") [result])
        )
    "#,
        swarms[0].peer_id
    );

    let particle = Particle {
        id: uuid(),
        init_peer_id: swarms[1].peer_id,
        timestamp: now_ms() as u64,
        ttl: PARTICLE_TTL,
        script,
        signature: vec![],
        data: vec![],
    };

    let exec_f = swarms[1].aquamarine_api.clone().execute(particle, None);

    let result = block_on(timeout(Duration::from_secs(30), async move {
        add_first_f.await;
        add_second_f.await;
        exec_f.await;
        inlet.await
    }));

    println!("result: {:?}", result);
}
