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

#![recursion_limit = "512"]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

use async_std::task;
use ctrlc_adapter::block_until_ctrlc;
use futures::channel::oneshot;
use janus_ipfs::run_ipfs_multiaddr_service;
use parity_multiaddr::Multiaddr;
use std::error::Error;
use std::io::Write;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {} {} JI]: {}",
                buf.timestamp_micros(),
                record.level(),
                record.module_path().unwrap_or_default(),
                record.args()
            )
        })
        .init();

    // TODO: use ArgMatches
    let bootstrap: Multiaddr = std::env::args()
        .nth(1)
        .expect("multiaddr for bootstrap node should be provided by the first argument")
        .parse()
        .expect("provided wrong bootstrap Multiaddr");

    let ipfs: Multiaddr = std::env::args()
        .nth(2)
        .expect("ipfs multiaddr should be provided as a second argument")
        .parse()
        .expect("provided wrong IPFS Multiaddr");

    let (exit_sender, exit_receiver) = oneshot::channel::<()>();

    let ipfs_task = task::spawn(async move {
        let result = run_ipfs_multiaddr_service(bootstrap, ipfs, exit_receiver).await;
        if let Err(e) = result {
            log::error!("Error running ipfs_task: {:?}", e)
        }
    });

    log::info!("Waiting for Ctrl-C...");
    block_until_ctrlc();
    log::debug!("Got Ctrl-C, stopping");
    exit_sender.send(()).unwrap();
    log::debug!("Waiting for ipfs_task to end");
    task::block_on(ipfs_task);
    log::debug!("ipfs_task stopped");
    Ok(())
}
