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
use std::path::PathBuf;
use std::{collections::HashMap, ops::Deref, sync::Arc, time::Duration};

use avm_server::{
    AVMConfig, AVMOutcome, CallRequestParams, CallResults, CallServiceResult, IValue, AVM,
};
use fstrings::f;
use libp2p::PeerId;
use parking_lot::Mutex;
use serde_json::{Value as JValue, Value};

use air_interpreter_fs::{air_interpreter_path, write_default_air_interpreter};
use aquamarine::{DataStoreError, ParticleDataStore};
use fs_utils::make_tmp_dir;
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
            i => panic!("Didn't expect instruction to be {:?}", i),
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
        ("return", _) | ("op", "return") | ("callbackSrv", "response") => {
            log::info!("op return: {:?}", args.function_args);
            ClientFunctionsResult {
                outcome: FunctionOutcome::Empty,
                returned: Some(Ok(args.function_args)),
            }
        }
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
        (service, function) => {
            let error = f!("service not found: {service} {function}");
            println!("{}", error);
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
    log::info!("host_call result: {:?}", result);
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

pub fn make_vm(peer_id: PeerId) -> AVM<DataStoreError> {
    let tmp = make_tmp_dir();
    let interpreter = air_interpreter_path(&tmp);
    write_default_air_interpreter(&interpreter).expect("write air interpreter");

    let particle_data_store: PathBuf = format!("/tmp/{}", peer_id.to_string()).into();
    let vault_dir = particle_data_store.join("vault");
    let data_store = Box::new(ParticleDataStore::new(particle_data_store, vault_dir));
    let config = AVMConfig {
        data_store,
        current_peer_id: peer_id.to_base58(),
        air_wasm_path: interpreter,
        logging_mask: i32::MAX,
    };

    AVM::new(config)
        .map_err(|err| {
            log::error!("\n\n\nFailed to create local AVM: {:#?}\n\n\n", err);

            println!("\n\n\nFailed to create local AVM: {:#?}\n\n\n", err);

            err
        })
        .expect("vm should be created")
}

pub fn wrap_script(
    script: String,
    data: &HashMap<String, JValue>,
    relay: impl Into<Option<PeerId>>,
    generated: bool,
    executor: Option<PeerId>,
) -> String {
    let executor = executor
        .map(|p| format!("\"{}\"", p))
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

pub fn make_particle(
    peer_id: PeerId,
    service_in: &HashMap<String, JValue>,
    script: String,
    relay: impl Into<Option<PeerId>>,
    local_vm: &mut AVM<DataStoreError>,
    generated: bool,
    particle_ttl: Duration,
) -> Particle {
    let script = wrap_script(script, service_in, relay, generated, None);

    let id = uuid();

    let mut call_results: CallResults = <_>::default();
    let mut particle_data = vec![];
    loop {
        let AVMOutcome {
            data,
            next_peer_pks,
            call_requests,
        } = local_vm
            .call(
                script.clone(),
                particle_data,
                peer_id.to_string(),
                &id,
                call_results,
            )
            .expect("execute & make particle");

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
    }

    log::info!("Made a particle {}", id);

    Particle {
        id,
        init_peer_id: peer_id,
        timestamp: now_ms() as u64,
        ttl: particle_ttl.as_millis() as u32,
        script,
        signature: vec![],
        data: particle_data,
    }
}

pub fn read_args(
    particle: Particle,
    peer_id: PeerId,
    local_vm: &mut AVM<DataStoreError>,
) -> Result<Vec<JValue>, Vec<JValue>> {
    let mut call_results: CallResults = <_>::default();
    let mut particle_data = particle.data;
    loop {
        let AVMOutcome {
            data,
            next_peer_pks,
            call_requests,
        } = local_vm
            .call(
                &particle.script,
                particle_data,
                particle.init_peer_id.to_string(),
                &particle.id,
                call_results,
            )
            .expect("execute & make particle");

        particle_data = data;
        call_results = <_>::default();

        if call_requests.is_empty() {
            return Ok(vec![]);
        }
        for (id, call) in call_requests {
            let args = Args::try_from(call).expect("valid args");
            let result = host_call(&<_>::default(), args);
            call_results.insert(id, result.0);

            if result.1.is_some() {
                return result.1.unwrap();
            }
        }
    }
}
