/*
 * Copyright 2021 Fluence Labs Limited
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

use std::borrow::Cow;
use std::collections::HashMap;
use std::task::Poll;

use async_std::sync::Arc;
use avm_server::{CallRequestParams, CallRequests, CallResults, CallServiceResult};
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use serde_json::Value as JValue;

use host_closure::{Args, JError};
use particle_protocol::Particle;

use parking_lot::Mutex;
use particle_functions::particle_params::ParticleParams;
use std::convert::TryFrom;
use std::time::Instant;

type ParticleId = String;
type FunctionDuplet = (Cow<'static, str>, Cow<'static, str>);
type Output = BoxFuture<'static, Result<Option<JValue>, JError>>;
pub type Function = Arc<Mutex<Box<dyn HostFunction>>>;

pub trait HostFunction: Clone {
    fn call(&self, args: Args, particle: ParticleParams) -> Output;
    fn call_mut(&mut self, args: Args, particle: ParticleParams) -> Output;
}

pub struct Functions<F> {
    particle: ParticleParams,
    host_functions: F,
    function_calls: FuturesUnordered<(u32, CallServiceResult)>,
    call_results: CallResults,
    particle_functions: HashMap<FunctionDuplet, Function>,
}

impl<F: HostFunction> Functions<F> {
    pub fn new(particle: ParticleParams, host_functions: F) -> Self {
        Self {
            particle,
            host_functions,
            function_calls: <_>::default(),
            call_results: <_>::default(),
            particle_functions: <_>::default(),
        }
    }

    /// Advance call requests execution
    pub fn poll(&mut self) {
        while let Poll::Ready(Some((id, result))) = self.function_calls.poll_next_unpin() {
            let overwritten = self.call_results.insert(id, result).is_none();

            debug_assert!(
                overwritten.is_none(),
                "unreachable: function call {} overwritten",
                id
            );
        }
    }

    /// Add a bunch of call requests to execution
    pub fn execute(&mut self, requests: CallRequests) {
        let futs = requests.into_iter().map(|(id, call)| self.call(id, call));
        self.function_calls.extend(futs);
        self.poll();
    }

    /// Retrieve all existing call results
    pub fn drain(&mut self) -> CallResults {
        std::mem::replace(&mut self.call_results, <_>::default())
    }

    // TODO: currently AFAIK there's no cooperation between tasks/executors because all futures
    //       are executed inside `block_on`.
    //       i.e., if one future yields, it blocks the whole thread (does it? I'm not sure)
    //       Probably, the situation can be improved by somehow executing all futures in a cooperative manner.
    //       I see the main obstacle to cooperation in streaming results to `self.call_results`.
    //       Streaming can be done through an MPSC channel, but it seems like an overkill. Though
    //       maybe it's a good option.
    async fn call(&self, id: u32, call: CallRequestParams) -> (u32, CallServiceResult) {
        use async_std::task::{block_on, spawn_blocking};
        use Cow::Borrowed;

        // Deserialize params
        let args = match Args::try_from(call) {
            Ok(args) => args,
            Err(err) => {
                return (
                    id,
                    CallServiceResult {
                        ret_code: 1,
                        result: json!(format!(
                            "Failed to deserialize CallRequestParams to Args: {}",
                            err
                        ))
                        .to_string(),
                    },
                )
            }
        };

        log::trace!("Host function call, args: {:#?}", args);
        let log_args = format!("{:?} {:?}", args.service_id, args.function_name);

        let start = Instant::now();

        let params = self.particle.clone();
        // Check if a user-defined callback fits
        let duplet: FunctionDuplet = (Borrowed(&call.service_id), Borrowed(&call.function_name));
        let result = if let Some(func) = self.particle_functions.get(&duplet) {
            let func = func.clone();
            // Move to blocking threadpool 'cause user-defined callback may use blocking calls
            spawn_blocking(move || {
                // TODO: Actors would allow to get rid of Mutex
                //       i.e., wrap each callback with a queue & channel
                let mut func = func.lock();
                block_on(func.call_mut(args, params).await)
            })
            .await
        } else {
            // Move to blocking threadpool 'cause particle_closures::HostFunctions
            // uses std::fs and parking_lot which are blocking
            spawn_blocking(move || task::block_on(self.host_functions.call(args, params))).await
        };

        let elapsed = pretty(start.elapsed());
        if let Err(err) = &result {
            log::warn!("Failed host call {} ({}): {}", log_args, elapsed, err)
        } else {
            log::info!("Executed host call {} ({})", log_args, elapsed);
        };

        let result = match result {
            Ok(v) => CallServiceResult {
                ret_code: 0,
                result: v.map_or(json!(""), |v| json!(v)).to_string(),
            },
            Err(e) => CallServiceResult {
                ret_code: 1,
                result: json!(JValue::from(e)).to_string(),
            },
        };

        (id, result)
    }
}
