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

use std::convert::TryFrom;
use std::ops::Try;
use std::path::Path;
use std::sync::Arc;
use std::{collections::HashMap, time::Duration};

use avm_server::avm_runner::{AVMRunner, RawAVMOutcome};
use avm_server::{CallResults, CallServiceResult};
use fstrings::f;
use libp2p::PeerId;
use serde_json::{json, Value as JValue};

use air_interpreter_fs::{air_interpreter_path, write_default_air_interpreter};
use aquamarine::ParticleDataStore;
use fluence_keypair::KeyPair;
use now_millis::now_ms;
use particle_args::{Args, JError};
use particle_execution::FunctionOutcome;
use particle_protocol::Particle;
use uuid_utils::uuid;

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction {
    Seq(Box<Instruction>, Box<Instruction>),
    Call(String),
    Null,
}

impl Instruction {
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, call: String) -> Self {
        use Instruction::*;

        let call = move || Seq(Call(call).into(), Null.into());
        match self {
            Null => call(),
            Seq(left, r) if *r == Null => Seq(left, call().into()),
            s @ Seq(..) => Seq(s.into(), call().into()),
            i => panic!("Didn't expect instruction to be {i:?}"),
        }
    }

    pub fn into_air(self) -> String {
        use Instruction::*;

        match self {
            Null => "(null)".to_string(),
            Call(call) => call,
            Seq(l, r) => {
                let l = l.into_air();
                let r = r.into_air();
                f!("(seq 
{l}
{r}
)")
            }
        }
    }
}

pub type Returned = Option<Result<Vec<JValue>, Vec<JValue>>>;

#[derive(Debug)]
pub struct ClientFunctionsResult {
    pub outcome: FunctionOutcome,
    pub returned: Returned,
}

pub fn client_functions(data: &HashMap<String, JValue>, args: Args) -> ClientFunctionsResult {
    match (args.service_id.as_str(), args.function_name.as_str()) {
        ("load", _) | ("getDataSrv", _) => {
            let value = data.get(args.function_name.as_str()).cloned();
            let outcome = match value {
                Some(v) => FunctionOutcome::Ok(v),
                None => FunctionOutcome::Err(JError::new(f!(
                    "variable not found: {args.function_name}"
                ))),
            };
            ClientFunctionsResult {
                outcome,
                returned: None,
            }
        }
        ("return", _) | ("op", "return") | ("callbackSrv", "response") => ClientFunctionsResult {
            outcome: FunctionOutcome::Empty,
            returned: Some(Ok(args.function_args)),
        },
        ("callbackSrv", _) => {
            log::warn!("got callback: {:?}", args.function_args);
            ClientFunctionsResult {
                outcome: FunctionOutcome::Empty,
                returned: None,
            }
        }
        ("errorHandlingSrv", "error") => {
            log::warn!("caught an error: {:?}", args.function_args);
            ClientFunctionsResult {
                outcome: FunctionOutcome::Empty,
                returned: Some(Err(args.function_args)),
            }
        }
        (_, "identity") => ClientFunctionsResult {
            outcome: FunctionOutcome::from_output(args.function_args.into_iter().next()),
            returned: None,
        },
        ("op", "noop") => ClientFunctionsResult {
            outcome: FunctionOutcome::Empty,
            returned: None,
        },
        ("run-console", "print") => {
            println!("run-console: {}", json!(args.function_args));
            log::info!("run-console: {}", json!(args.function_args));
            ClientFunctionsResult {
                outcome: FunctionOutcome::Empty,
                returned: None,
            }
        }
        (service, function) => {
            let error = f!("service not found: {service} {function}");
            println!("{error}");
            log::warn!("{}", error);
            ClientFunctionsResult {
                outcome: FunctionOutcome::Err(JError::new(error)),
                returned: None,
            }
        }
    }
}

pub fn host_call(data: &HashMap<String, JValue>, args: Args) -> (CallServiceResult, Returned) {
    let result = client_functions(data, args);
    let outcome = result.outcome;
    let outcome = match outcome {
        FunctionOutcome::NotDefined { args, .. } => Err(JError::new(format!(
            "Service with id '{}' not found (function {})",
            args.service_id, args.function_name
        ))),
        FunctionOutcome::Empty => Ok(JValue::String(String::new())),
        FunctionOutcome::Ok(v) => Ok(v),
        FunctionOutcome::Err(err) => Err(err),
    };

    let outcome = match outcome {
        Ok(result) => CallServiceResult {
            ret_code: 0,
            result,
        },
        Err(err) => CallServiceResult {
            ret_code: 1,
            result: JValue::from(err),
        },
    };

    (outcome, result.returned)
}

pub fn make_vm(tmp_dir_path: &Path) -> AVMRunner {
    let interpreter = air_interpreter_path(tmp_dir_path);
    write_default_air_interpreter(&interpreter).expect("write air interpreter");

    let runner = AVMRunner::new(interpreter, None, i32::MAX)
        .map_err(|err| {
            log::error!("\n\n\nFailed to create local AVM: {:#?}\n\n\n", err);

            println!("\n\n\nFailed to create local AVM: {err:#?}\n\n\n");

            err
        })
        .expect("vm should be created");
    runner
}

pub fn wrap_script(
    script: String,
    data: &HashMap<String, JValue>,
    relay: impl Into<Option<PeerId>>,
    generated: bool,
    executor: Option<PeerId>,
) -> String {
    let executor = executor
        .map(|p| format!("\"{p}\""))
        .unwrap_or(String::from("%init_peer_id%"));

    let load_variables = if generated {
        "   (null)".to_string()
    } else {
        data.keys()
            .map(|name| f!(r#"  (call {executor} ("load" "{name}") [] {name})"#))
            .fold(Instruction::Null, |acc, call| acc.add(call))
            .into_air()
    };

    let catch = f!(r#"(call {executor} ("errorHandlingSrv" "error") [%last_error%])"#);
    let catch = if let Some(relay) = relay.into() {
        f!(r#"
        (seq
            (call "{relay}" ("op" "identity") [])
            {catch}
        )
        "#)
    } else {
        catch
    };

    let script = f!(r#"
(seq
{load_variables}
    (xor
        {script}
        {catch}
    )
)
    "#);

    script
}

#[allow(clippy::too_many_arguments)]
pub async fn make_particle(
    peer_id: PeerId,
    service_in: &HashMap<String, JValue>,
    script: String,
    relay: impl Into<Option<PeerId>>,
    local_vm: &mut AVMRunner,
    data_store: Arc<ParticleDataStore>,
    generated: bool,
    particle_ttl: Duration,
    key_pair: &KeyPair,
) -> Particle {
    let script = wrap_script(script, service_in, relay, generated, None);

    let id = uuid();
    let timestamp = now_ms() as u64;
    let ttl = particle_ttl.as_millis() as u32;

    let mut call_results: CallResults = <_>::default();
    let mut particle_data = vec![];
    loop {
        let RawAVMOutcome {
            data,
            call_requests,
            ..
        } = local_vm
            .call(
                &script,
                vec![],
                particle_data,
                peer_id.to_string(),
                timestamp,
                ttl,
                peer_id.to_string(),
                call_results,
                key_pair,
                id.clone(),
            )
            .expect("execute & make particle");

        data_store
            .store_data(&data, id.as_str(), peer_id.to_base58().as_str())
            .await
            .expect("Could not store data");

        particle_data = data;
        call_results = <_>::default();

        if call_requests.is_empty() {
            break;
        }
        for (id, call) in call_requests {
            let args = Args::try_from(call).expect("valid args");
            let result = host_call(service_in, args);
            call_results.insert(id, result.0);
        }
        tokio::task::yield_now().await;
    }

    let mut particle = Particle {
        id: id.clone(),
        init_peer_id: peer_id,
        timestamp,
        ttl,
        script,
        signature: vec![],
        data: particle_data,
    };

    particle.sign(key_pair).expect("sign particle");

    tracing::info!(
        particle_id = id,
        "Made a particle with timestamp = {}, TTL = {}",
        particle.ttl,
        particle.timestamp
    );

    particle
}

pub async fn read_args(
    particle: Particle,
    peer_id: PeerId,
    local_vm: &mut AVMRunner,
    data_store: Arc<ParticleDataStore>,
    key_pair: &KeyPair,
) -> Option<Result<Vec<JValue>, Vec<JValue>>> {
    let mut call_results: CallResults = <_>::default();
    let mut particle_data = particle.data;
    loop {
        let prev_data = data_store
            .read_data(particle.id.as_str(), peer_id.to_base58().as_str())
            .await
            .expect("Could not load prev data");

        let RawAVMOutcome {
            data,
            call_requests,
            ..
        } = local_vm
            .call(
                &particle.script,
                prev_data,
                particle_data,
                particle.init_peer_id.to_string(),
                particle.timestamp,
                particle.ttl,
                peer_id.to_string(),
                call_results,
                key_pair,
                particle.id.clone(),
            )
            .expect("execute & make particle");
        data_store
            .store_data(&data, particle.id.as_str(), peer_id.to_base58().as_str())
            .await
            .expect("Could not store data");

        particle_data = data;
        call_results = <_>::default();

        if call_requests.is_empty() {
            return None;
        }
        for (id, call) in call_requests {
            let args = Args::try_from(call).expect("valid args");
            let (call_result, returned) = host_call(&<_>::default(), args);
            call_results.insert(id, call_result);

            if returned.is_some() {
                return returned;
            }
        }
        tokio::task::yield_now().await;
    }
}
