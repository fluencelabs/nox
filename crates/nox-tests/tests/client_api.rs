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

use std::time::Duration;

use futures::channel::oneshot::channel;
use futures::future::BoxFuture;
use futures::FutureExt;
use maplit::hashmap;
use serde_json::json;
use tracing::Span;

use created_swarm::make_swarms;
use now_millis::now_ms;
use particle_execution::FunctionOutcome;
use particle_protocol::{ExtendedParticle, Particle};
use test_constants::PARTICLE_TTL;
use test_utils::timeout;
use uuid_utils::uuid;

#[tokio::test]
async fn call_custom_service() {
    let swarms = make_swarms(2).await;

    // pub type Output<'a> = BoxFuture<'a, FunctionOutcome>;
    //
    // pub type ServiceFunction =
    // Box<dyn FnMut(Args, ParticleParams) -> Output<'static> + 'static + Send + Sync>;

    let closure: Box<dyn Fn(_, _) -> BoxFuture<'static, FunctionOutcome> + 'static + Send + Sync> =
        Box::new(move |args, params| {
            async move {
                println!("got call!!! {args:?} {params:?}");
                FunctionOutcome::Ok(json!("hello!"))
            }
            .boxed()
        });

    let add_first_f = swarms[0].aquamarine_api.clone().add_service(
        "hello".into(),
        hashmap! { "world".to_string() => closure.into() },
    );

    let (outlet, inlet) = channel();
    let mut outlet = Some(outlet);
    let closure: Box<
        dyn FnMut(_, _) -> BoxFuture<'static, FunctionOutcome> + 'static + Send + Sync,
    > = Box::new(move |args, params| {
        let mut outlet = outlet.take();
        async move {
            let outlet = outlet.take();
            println!("got return call!!! {args:?} {params:?}");
            outlet.map(|out| out.send((args, params)));
            FunctionOutcome::Empty
        }
        .boxed()
    });

    let add_second_f = swarms[1].aquamarine_api.clone().add_service(
        "op".into(),
        hashmap! { "return".to_string() => closure.into() },
    );

    let script = format!(
        r#"
        (seq
            (call "{}" ("hello" "world") [] result)
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

    let exec_f = swarms[1]
        .aquamarine_api
        .clone()
        .execute(ExtendedParticle::new(particle, Span::none()), None);

    let result = timeout(Duration::from_secs(30), async move {
        add_first_f.await.expect("add_first_f");
        add_second_f.await.expect("add_second_f");
        exec_f.await.expect("exec_f");
        inlet.await
    })
    .await;

    println!("result: {result:?}");
}
