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

use std::{thread::sleep, time::Duration};

use eyre::WrapErr;

use connected_client::ConnectedClient;
use created_swarm::make_swarms_with_cfg;
use test_constants::KAD_TIMEOUT;

pub fn connect_swarms(node_count: usize) -> impl Fn(usize) -> ConnectedClient {
    let swarms = make_swarms_with_cfg(node_count, |mut cfg| {
        cfg.pool_size = Some(3);
        cfg
    });
    sleep(KAD_TIMEOUT);

    move |i| ConnectedClient::connect_to(swarms[i].multiaddr.clone()).expect("connect client")
}

pub async fn timeout<F, T>(dur: Duration, f: F) -> eyre::Result<T>
where
    F: std::future::Future<Output = T>,
{
    Ok(async_std::future::timeout(dur, f)
        .await
        .wrap_err(format!("timed out after {:?}", dur))?)
}
