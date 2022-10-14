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

use std::convert::TryFrom;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use avm_server::{CallRequestParams, CallRequests, CallResults, CallServiceResult};
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use humantime::format_duration as pretty;
use parking_lot::Mutex;
use serde_json::json;
use serde_json::Value as JValue;

use particle_args::{Args, JError};
use particle_execution::{
    FunctionOutcome, ParticleFunctionStatic, ParticleParams, ServiceFunction,
};
use peer_metrics::FunctionKind;

#[derive(Clone, Debug)]
/// Performance statistics about executed function call
pub struct SingleCallStat {
    /// If execution happened, then how much time it took
    pub run_time: Option<Duration>,
    pub success: bool,
    /// Whether function call was to builtin functions (like op noop) or to services
    pub kind: FunctionKind,
}

#[derive(Clone, Debug)]
pub struct SingleCallResult {
    call_id: u32,
    result: CallServiceResult,
    stat: SingleCallStat,
}

pub struct Functions<F> {
    particle: ParticleParams,
    builtins: Arc<F>,
    function_calls: FuturesUnordered<BoxFuture<'static, SingleCallResult>>,
    call_results: CallResults,
    call_stats: Vec<SingleCallStat>,
    particle_function: Option<Arc<Mutex<ServiceFunction>>>,
}

impl<F: ParticleFunctionStatic> Functions<F> {
    pub fn new(particle: ParticleParams, builtins: Arc<F>) -> Self {
        Self {
            particle,
            builtins,
            function_calls: <_>::default(),
            call_results: <_>::default(),
            call_stats: <_>::default(),
            particle_function: None,
        }
    }

    /// Advance call requests execution
    pub fn poll(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(r)) = self.function_calls.poll_next_unpin(cx) {
            let overwritten = self.call_results.insert(r.call_id, r.result);
            self.call_stats.push(r.stat);

            debug_assert!(
                overwritten.is_none(),
                "unreachable: function call result {} was overwritten",
                r.call_id
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
    pub fn drain(&mut self) -> (CallResults, Vec<SingleCallStat>) {
        let call_results = std::mem::take(&mut self.call_results);
        let stats = std::mem::take(&mut self.call_stats);

        (call_results, stats)
    }

    pub fn set_function(&mut self, function: ServiceFunction) {
        self.particle_function = Some(Arc::new(Mutex::new(function)));
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
        call_id: u32,
        call: CallRequestParams,
        waker: Waker,
    ) -> BoxFuture<'static, SingleCallResult> {
        use async_std::task::{block_on, spawn_blocking};

        // Deserialize params
        let args = match Args::try_from(call) {
            Ok(args) => args,
            Err(err) => {
                let result = CallServiceResult {
                    ret_code: 1,
                    result: json!(format!(
                        "Failed to deserialize CallRequestParams to Args: {}",
                        err
                    )),
                };
                let result = SingleCallResult {
                    call_id,
                    result,
                    stat: SingleCallStat {
                        run_time: None,
                        success: false,
                        kind: FunctionKind::NotHappened,
                    },
                };
                return async move { result }.boxed();
            }
        };

        let log_args = format!("{:?} {:?}", args.service_id, args.function_name);

        let start = Instant::now();

        let params = self.particle.clone();
        let builtins = self.builtins.clone();
        let particle_function = self.particle_function.clone();

        let result = spawn_blocking(move || {
            block_on(async move {
                let outcome = builtins.call(args, params).await;
                // record whether call was handled by builtin or not. needed for stats.
                let mut call_kind = FunctionKind::Service;
                let outcome = match outcome {
                    // If particle_function isn't set, just return what we have
                    outcome if particle_function.is_none() => outcome,
                    // If builtins weren't defined over these args, try particle_function
                    FunctionOutcome::NotDefined { args, params } => {
                        let func = particle_function.unwrap();
                        // TODO: Actors would allow to get rid of Mutex
                        //       i.e., wrap each callback with a queue & channel
                        let mut func = func.lock();
                        let outcome = func(args, params).await;
                        call_kind = FunctionKind::ParticleFunction;
                        outcome
                    }
                    // Builtins were called, return their outcome
                    outcome => outcome,
                };
                (outcome, call_kind)
            })
        });

        async move {
            let (result, call_kind) = result.await;
            let elapsed = start.elapsed();

            let result = match result {
                FunctionOutcome::NotDefined { args, .. } => Err(JError::new(format!(
                    "Service with id '{}' not found (function {})",
                    args.service_id, args.function_name
                ))),
                FunctionOutcome::Empty => Ok(JValue::String(String::new())),
                FunctionOutcome::Ok(v) => Ok(v),
                FunctionOutcome::Err(err) => Err(err),
            };

            if let Err(err) = &result {
                let elapsed = pretty(elapsed);
                log::warn!("Failed host call {} ({}): {}", log_args, elapsed, err)
            } else {
                log::info!("Executed host call {} ({})", log_args, pretty(elapsed));
            };

            let stats = SingleCallStat {
                run_time: Some(elapsed),
                success: result.is_ok(),
                kind: call_kind,
            };

            let result = match result {
                Ok(result) => CallServiceResult {
                    ret_code: 0,
                    result,
                },
                Err(err) => CallServiceResult {
                    ret_code: 1,
                    result: JValue::from(err),
                },
            };

            waker.wake();

            SingleCallResult {
                call_id,
                result,
                stat: stats,
            }
        }
        .boxed()
    }
}
