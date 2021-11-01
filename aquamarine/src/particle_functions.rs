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
use std::convert::TryFrom;
use std::task::{Context, Poll, Waker};
use std::time::Instant;

use async_std::sync::Arc;
use avm_server::{CallRequestParams, CallRequests, CallResults, CallServiceResult};
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use humantime::format_duration as pretty;
use parking_lot::Mutex;
use serde_json::json;
use serde_json::Value as JValue;

use particle_args::Args;
use particle_execution::{ParticleFunctionMut, ParticleFunctionStatic, ParticleParams};

type FunctionDuplet<'a> = (Cow<'a, str>, Cow<'a, str>);
pub type Function = Arc<Mutex<Box<dyn ParticleFunctionMut>>>;

pub struct Functions<F> {
    particle: ParticleParams,
    builtins: Arc<F>,
    function_calls: FuturesUnordered<BoxFuture<'static, (u32, CallServiceResult)>>,
    call_results: CallResults,
    particle_functions: HashMap<FunctionDuplet<'static>, Function>,
}

impl<F: ParticleFunctionStatic> Functions<F> {
    pub fn new(particle: ParticleParams, builtins: Arc<F>) -> Self {
        Self {
            particle,
            builtins,
            function_calls: <_>::default(),
            call_results: <_>::default(),
            particle_functions: <_>::default(),
        }
    }

    /// Advance call requests execution
    pub fn poll(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some((id, result))) = self.function_calls.poll_next_unpin(cx) {
            let overwritten = self.call_results.insert(id, result);

            debug_assert!(
                overwritten.is_none(),
                "unreachable: function call {} overwritten",
                id
            );
        }
    }

    /// Add a bunch of call requests to execution
    pub fn execute(&mut self, requests: CallRequests, waker: Waker) {
        let futs: Vec<_> = requests
            .into_iter()
            .map(|(id, call)| self.call(id, call, waker.clone()))
            .collect();
        self.function_calls.extend(futs);
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
    fn call(
        &self,
        id: u32,
        call: CallRequestParams,
        waker: Waker,
    ) -> BoxFuture<'static, (u32, CallServiceResult)> {
        use async_std::task::{block_on, spawn_blocking};
        use Cow::Borrowed;

        // Deserialize params
        let args = match Args::try_from(call) {
            Ok(args) => args,
            Err(err) => {
                let result = CallServiceResult {
                    ret_code: 1,
                    result: json!(format!(
                        "Failed to deserialize CallRequestParams to Args: {}",
                        err
                    ))
                    .to_string(),
                };
                return async move { (id, result) }.boxed();
            }
        };

        log::trace!("Host function call, args: {:#?}", args);
        let log_args = format!("{:?} {:?}", args.service_id, args.function_name);

        let start = Instant::now();

        let params = self.particle.clone();
        // Check if a user-defined callback fits
        let duplet: FunctionDuplet<'_> =
            (Borrowed(&args.service_id), Borrowed(&args.function_name));
        let result = if let Some(func) = self.particle_functions.get(&duplet) {
            let func = func.clone();
            // Move to blocking threadpool 'cause user-defined callback may use blocking calls
            spawn_blocking(move || {
                // TODO: Actors would allow to get rid of Mutex
                //       i.e., wrap each callback with a queue & channel
                let mut func = func.lock();
                block_on(func.call_mut(args, params))
            })
        } else {
            let builtins = self.builtins.clone();
            // Move to blocking threadpool 'cause particle_closures::ParticleFunctions
            // uses std::fs and parking_lot which are blocking
            spawn_blocking(move || block_on(builtins.call(args, params)))
        };

        let elapsed = pretty(start.elapsed());

        async move {
            let result = result.await;

            waker.wake();

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
        .boxed()
    }
}
