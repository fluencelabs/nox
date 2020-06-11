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

use async_std::task;
use fluence_client::Transport;
use fluence_server::Server;
use libp2p::core::multiaddr::{Multiaddr, Protocol};
use prometheus::Registry;
use rand::prelude::*;
use std::env;
use std::net::IpAddr;

use crate::utils::*;
use std::thread::sleep;

mod utils;

#[test]
#[ignore]
fn main() {
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

    let registry = Registry::new();

    start_bunch(count, port, host, bs_port, bs_max, Some(&registry));

    log::info!("started /metrics at {}:{}", host, port - 1);
    task::block_on(Server::start_metrics_endpoint(registry, (host, port - 1)))
        .expect("Start /metrics endpoint");
}

fn start_bunch(
    count: usize,
    port: u16,
    host: IpAddr,
    bs_port: Option<u16>,
    bs_max: usize,
    registry: Option<&Registry>,
) {
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
            create_swarm(bs, maddr, None, Transport::Network, registry)
        },
        || {
            let maddr = create_maddr(host, port + idx);
            idx += 1;
            maddr
        },
        false,
    );
}

fn create_maddr(host: IpAddr, port: u16) -> Multiaddr {
    let mut maddr = Multiaddr::from(host);
    maddr.push(Protocol::Tcp(port));
    maddr
}

#[test]
fn receive_call_on_big_network() {
    use rand::distributions::Alphanumeric;

    let localhost = "127.0.0.1".parse().unwrap();
    let count = 15;
    start_bunch(count, 20000, localhost, None, count, None);
    start_bunch(count, 30000, localhost, Some(20000), count, None);
    start_bunch(count, 40000, localhost, Some(30000), count, None);

    sleep(TIMEOUT * 2);

    let mut rng = thread_rng();
    let mut addr = |port| {
        format!("/ip4/127.0.0.1/tcp/{}", rng.gen_range(port, port + count))
            .parse()
            .unwrap()
    };

    let mut i = 10;
    while i > 0 {
        i -= 1;

        let service = thread_rng()
            .sample_iter(Alphanumeric)
            .take(7)
            .collect::<String>();
        let service = service.as_str();

        let mut client20 = ConnectedClient::connect_to(addr(20000)).expect("connect 20");
        let client30 = ConnectedClient::connect_to(addr(30000)).expect("connect 30");
        let client40 = ConnectedClient::connect_to(addr(40000)).expect("connect 40");

        log::info!("\niteration {}", i);
        log::info!("service: {}", service);
        log::info!("client20: {}", client20.relay_address());
        log::info!("client30: {}", client30.relay_address());
        log::info!("client40: {}", client40.relay_address());

        sleep(SHORT_TIMEOUT);

        client20.send(provide_call(service, client20.relay_address()));
        sleep(KAD_TIMEOUT);

        client30.send(service_call(service, client30.relay_address()));
        let call30to20 = client20.receive();
        log::info!("call30to20: {:?}", call30to20);

        client40.send(service_call(service, client40.relay_address()));
        let call40to20 = client20.receive();
        log::info!("call40to20: {:?}", call40to20);
    }
}
