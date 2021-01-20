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

use futures::future::BoxFuture;
use futures::FutureExt;
use prometheus::Registry;
use std::io;
use std::net::SocketAddr;

pub fn start_metrics_endpoint(
    registry: Registry,
    listen_addr: SocketAddr,
) -> BoxFuture<'static, io::Result<()>> {
    use prometheus::{Encoder, TextEncoder};
    use tide::{Error, StatusCode::InternalServerError};

    let mut app = tide::with_state(registry);
    app.at("/metrics")
        .get(|req: tide::Request<Registry>| async move {
            let mut buffer = vec![];
            let encoder = TextEncoder::new();
            let metric_families = req.state().gather();

            encoder
                .encode(&metric_families, &mut buffer)
                .map_err(|err| {
                    let msg = format!("Error encoding prometheus metrics: {:?}", err);
                    log::warn!("{}", msg);
                    Error::from_str(InternalServerError, msg)
                })?;

            String::from_utf8(buffer).map_err(|err| {
                let msg = format!("Error encoding prometheus metrics: {:?}", err);
                log::warn!("{}", msg);
                Error::from_str(InternalServerError, msg)
            })
        });

    app.listen(listen_addr).boxed()
}
