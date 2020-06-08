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

#![allow(unused_imports, dead_code)]

use fluence_client::Transport;
use fluence_server::Server;
use prometheus::Registry;

use crate::utils::*;
mod utils;

#[test]
#[ignore]
fn main() {
    use async_std::task;
    use libp2p::core::multiaddr::{Multiaddr, Protocol};
    use rand::prelude::*;
    use std::env;
    use std::net::IpAddr;

    env_logger::init();

    let count: usize = env::var("COUNT")
        .unwrap_or("10".into())
        .parse()
        .expect("count correct");

    let host: IpAddr = env::var("HOST")
        .unwrap_or("127.0.0.1".into())
        .parse()
        .expect("host correct");

    let port: u16 = env::var("PORT")
        .unwrap_or("2000".into())
        .parse()
        .expect("port correct");

    // Max number of bootstrap nodes
    let bs_max: usize = env::var("BS_MAX")
        .unwrap_or("10".into())
        .parse()
        .expect("bs correct");

    // Boostrap nodes will be HOST:BS_PORT..HOST:BS_PORT+BS_MAX
    let bs_port: Option<u16> = env::var("BS_PORT")
        .map(|s| s.parse().expect("bs correct"))
        .ok();

    fn create_maddr(host: IpAddr, port: u16) -> Multiaddr {
        let mut maddr = Multiaddr::from(host);
        maddr.push(Protocol::Tcp(port));
        maddr
    }

    let registry = Registry::new();

    let mut idx = 0;
    let mut rng = thread_rng();
    let external_bootstraps = bs_port.into_iter().flat_map(|p| {
        (p..p + bs_max as u16)
            .map(|p| create_maddr(host, p))
            .collect::<Vec<_>>()
    });

    make_swarms_with(
        count,
        |bs, maddr| {
            let rnd = bs.into_iter().choose_multiple(&mut rng, bs_max);
            let bs: Vec<_> = rnd.into_iter().chain(external_bootstraps.clone()).collect();
            create_swarm(bs, maddr, None, Transport::Network, Some(&registry))
        },
        || {
            let maddr = create_maddr(host, port + idx);
            idx += 1;
            maddr
        },
        false,
    );

    log::info!("started /metrics at {}:{}", host, port - 1);
    task::block_on(Server::start_metrics_endpoint(registry, (host, port - 1)))
        .expect("Start /metrics endpoint");
}
