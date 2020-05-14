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

use fluence_server::config::{certificates, create_args, load_config, FluenceConfig};
use fluence_server::Server;

use clap::App;
use ctrlc_adapter::block_until_ctrlc;
use futures::channel::oneshot;
use std::error::Error;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder().format_timestamp_micros().init();

    let arg_matches = App::new("Fluence Fluence protocol server")
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .args(create_args().as_slice())
        .get_matches();

    let fluence_config = load_config(arg_matches)?;

    let fluence = start_fluence(fluence_config)?;
    log::info!("Fluence has been successfully started.");

    log::info!("Waiting for Ctrl-C to exit...");
    block_until_ctrlc();

    log::info!("Shutting down...");
    fluence.stop();

    Ok(())
}

trait Stoppable {
    fn stop(self);
}

// NOTE: to stop Fluence just call Stoppable::stop()
fn start_fluence(config: FluenceConfig) -> Result<impl Stoppable, Box<dyn Error>> {
    log::trace!("starting Fluence");

    certificates::init(config.certificate_dir.as_str(), &config.root_key_pair)?;

    let key_pair = &config.root_key_pair.key_pair;
    log::info!(
        "public key = {}",
        bs58::encode(key_pair.public().encode().to_vec().as_slice()).into_string()
    );

    let node_service = Server::new(
        key_pair.clone(),
        config.server_config.clone(),
        config.root_weights.clone(),
    );

    let node_exit_outlet = node_service.start();

    struct Fluence {
        node_exit_outlet: oneshot::Sender<()>,
    }

    impl Stoppable for Fluence {
        fn stop(self) {
            // shutting down node service leads to shutting down peer service by canceling the mpsc channel
            self.node_exit_outlet.send(()).unwrap();
        }
    }

    Ok(Fluence { node_exit_outlet })
}
