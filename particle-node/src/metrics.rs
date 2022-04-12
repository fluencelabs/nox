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

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use futures::future::BoxFuture;
use futures::FutureExt;
use parking_lot::Mutex;
use prometheus_client::registry::Registry;

pub fn start_metrics_endpoint(
    registry: Registry,
    listen_addr: SocketAddr,
) -> BoxFuture<'static, io::Result<()>> {
    use prometheus_client::encoding::text::encode;
    use tide::{Error, StatusCode::InternalServerError};

    let registry = Arc::new(Mutex::new(registry));
    let mut app = tide::with_state(registry);
    app.at("/metrics")
        .get(|req: tide::Request<Arc<Mutex<Registry>>>| async move {
            let mut encoded = Vec::new();
            encode(&mut encoded, &req.state().lock()).map_err(|e| {
                let msg = format!("Error while text-encoding metrics: {}", e);
                log::warn!("{}", msg);
                Error::from_str(InternalServerError, msg)
            })?;
            let response = tide::Response::builder(200)
                .body(encoded)
                .content_type("text/plain; version=1.0.0; charset=utf-8")
                .build();
            Ok(response)
        });

    app.listen(listen_addr).boxed()
}
