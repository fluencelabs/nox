/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

#![recursion_limit = "512"]
#![warn(rust_2018_idioms)]
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
