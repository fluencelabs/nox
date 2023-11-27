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
use serde_json::json;
use serde_json::Value as JValue;
use tokio::runtime::Handle;
use tracing::{instrument, Instrument, Span};

use particle_args::{Args, JError};
use particle_execution::{
    FunctionOutcome, ParticleFunctionStatic, ParticleParams, ServiceFunction,
};
use peer_metrics::FunctionKind;

use crate::log::builtin_log_fn;

#[derive(Clone, Debug)]
/// Performance statistics about executed function call
pub struct SingleCallStat {
    /// If execution happened, then how much time it took to execute the call
    pub call_time: Option<Duration>,
    /// If execution happened, then how much time call waited to be scheduled on blocking pool
    pub wait_time: Option<Duration>,
    pub success: bool,
    /// Whether function call was to builtin functions (like op noop) or to services
    pub kind: FunctionKind,
}

#[derive(Clone, Debug)]
pub struct SingleCallResult {
    /// `call_id` comes from AVM's CallRequest
    call_id: u32,
    result: CallServiceResult,
    stat: SingleCallStat,
    span: Arc<Span>,
}

pub struct Functions<F> {
    particle: ParticleParams,
    builtins: F,
    function_calls: FuturesUnordered<BoxFuture<'static, SingleCallResult>>,
    call_results: CallResults,
    call_stats: Vec<SingleCallStat>,
    call_spans: Vec<Arc<Span>>,
    particle_function: Option<Arc<tokio::sync::Mutex<ServiceFunction>>>,
}

impl<F: ParticleFunctionStatic> Functions<F> {
    pub fn new(particle: ParticleParams, builtins: F) -> Self {
        Self {
            particle,
            builtins,
            function_calls: <_>::default(),
            call_results: <_>::default(),
            call_stats: <_>::default(),
            call_spans: <_>::default(),
            particle_function: None,
        }
    }

    /// Advance call requests execution
    pub fn poll(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(r)) = self.function_calls.poll_next_unpin(cx) {
            let overwritten = self.call_results.insert(r.call_id, r.result);
            self.call_stats.push(r.stat);
            self.call_spans.push(r.span);

            debug_assert!(
                overwritten.is_none(),
                "unreachable: function call result {} was overwritten",
                r.call_id
            );
        }
    }

    /// Add a bunch of call requests to execution
    #[instrument(level = tracing::Level::INFO, skip_all)]
    pub fn execute(
        &mut self,
        particle_id: String,
        requests: CallRequests,
        waker: Waker,
        span: Arc<Span>,
    ) {
        let futs: Vec<_> = requests
            .into_iter()
            .map(|(id, call)| self.call(particle_id.clone(), id, call, waker.clone(), span.clone()))
            .collect();
        self.function_calls.extend(futs);
    }

    /// Retrieve all existing call results
    pub fn drain(&mut self) -> (CallResults, Vec<SingleCallStat>, Vec<Arc<Span>>) {
        let call_results = std::mem::take(&mut self.call_results);
        let stats = std::mem::take(&mut self.call_stats);
        let call_spans = std::mem::take(&mut self.call_spans);

        (call_results, stats, call_spans)
    }

    pub fn set_function(&mut self, function: ServiceFunction) {
        self.particle_function = Some(Arc::new(tokio::sync::Mutex::new(function)));
    }

    // TODO: currently AFAIK there's no cooperation between tasks/executors because all futures
    //       are executed inside `block_on`.
    //       i.e., if one future yields, it blocks the whole thread (does it? I'm not sure)
    //       Probably, the situation can be improved by somehow executing all futures in a cooperative manner.
    //       I see the main obstacle to cooperation in streaming results to `self.call_results`.
    //       Streaming can be done through an MPSC channel, but it seems like an overkill. Though
    //       maybe it's a good option.
    #[instrument(level = tracing::Level::INFO, skip_all)]
    fn call(
        &self,
        particle_id: String,
        call_id: u32,
        call: CallRequestParams,
        waker: Waker,
        span: Arc<Span>,
    ) -> BoxFuture<'static, SingleCallResult> {
        let local_span = tracing::info_span!(parent: span.as_ref(), "Particle functions: call");
        let _guard = local_span.enter();
        // Deserialize params
        let args = match Args::try_from(call) {
            Ok(args) => args,
            Err(err) => {
                return async move {
                    let result = CallServiceResult {
                        ret_code: 1,
                        result: json!(format!(
                            "Failed to deserialize CallRequestParams to Args: {err}"
                        )),
                    };
                    SingleCallResult {
                        call_id,
                        result,
                        stat: SingleCallStat {
                            call_time: None,
                            wait_time: None,
                            success: false,
                            kind: FunctionKind::NotHappened,
                        },
                        span,
                    }
                }
                .in_current_span()
                .boxed();
            }
        };

        let log_args = format!(
            "{:?} {:?} {}",
            args.service_id,
            args.function_name,
            json!(&args.function_args)
        );
        let service_id = args.service_id.clone();

        let params = self.particle.clone();
        let builtins = self.builtins.clone();
        let particle_function = self.particle_function.clone();
        let schedule_wait_start = Instant::now();
        let result = tokio::task::Builder::new()
            .name(&format!(
                "Call function {}:{}",
                args.service_id, args.function_name
            ))
            .spawn_blocking(move || {
                Handle::current().block_on(async move {
                    // How much time it took to start execution on blocking pool
                    let schedule_wait_time = schedule_wait_start.elapsed();
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
                            let func = func.lock().await;
                            let outcome = func.call(args, params).await;
                            call_kind = FunctionKind::ParticleFunction;
                            outcome
                        }
                        // Builtins were called, return their outcome
                        outcome => outcome,
                    };
                    // How much time it took to execute the call
                    // TODO: Time for ParticleFunction includes lock time, which is not good. Low priority cuz ParticleFunctions are barely used.
                    let call_time = schedule_wait_start.elapsed() - schedule_wait_time;
                    (outcome, call_kind, call_time, schedule_wait_time)
                })
            })
            .expect("Could not spawn task");

        async move {
            let (result, call_kind, call_time, wait_time) =
                result.await.expect("Could not 'Call function' join");

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
                tracing::warn!(
                    particle_id = particle_id,
                    "Failed host call {} ({}): {}",
                    log_args,
                    pretty(call_time),
                    err
                )
            } else {
                builtin_log_fn(&service_id, &log_args, pretty(call_time), particle_id);
            };

            let stats = SingleCallStat {
                call_time: Some(call_time),
                wait_time: Some(wait_time),
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
                span,
            }
        }
        .in_current_span()
        .boxed()
    }
}
