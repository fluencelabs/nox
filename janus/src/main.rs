/*
 * Copyright 2019 Fluence Labs Limited
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

mod config;
mod error;
mod janus_service;
mod node_handler;
mod p2p;
mod relay;

use crate::config::JanusConfig;
use crate::janus_service::JanusService;
use crate::node_handler::message::NodeEvent;
use std::{thread, time};
use tokio::sync::mpsc;

fn main() {
    let janus_service = JanusService::new(JanusConfig::default());
    let (_node_send, node_recv) = mpsc::unbounded_channel::<NodeEvent>();
    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio Runtime");

    match janus_service::start_janus(janus_service, node_recv, &runtime.executor()) {
        Ok(janus_exit) => {
            println!("Janus has been successfully started");
            let ten_millis = time::Duration::from_secs(10);
            thread::sleep(ten_millis);

            println!("exiting");
            janus_exit.send(()).expect("failed Janus exiting");
        }
        Err(_) => println!("Error occurred during Janus service starting"),
    }
}
