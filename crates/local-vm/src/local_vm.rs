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
use host_closure::{Args, JError};
use now_millis::now_ms;
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

pub fn host_call(
    data: &HashMap<String, JValue>,
    call: CallRequestParams,
) -> Result<CallServiceResult, Vec<JValue>> {
    let args = Args::try_from(call).expect("valid args");
    let result: Result<Option<JValue>, JError> =
        match (args.service_id.as_str(), args.function_name.as_str()) {
            ("load", _) | ("getDataSrv", _) => data
                .get(args.function_name.as_str())
                .cloned()
                .ok_or_else(|| JError::new(f!("variable not found: {args.function_name}")))
                .map(Some),
            ("return", _) | ("op", "return") | ("callbackSrv", "response") => {
                println!("op return: {:?}", args.function_args);
                return Err(args.function_args);
            }
            ("callbackSrv", _) => {
                log::warn!("got callback: {:?}", args.function_args);
                Ok(None)
            }
            ("errorHandlingSrv", "error") => {
                log::warn!("caught an error: {:?}", args.function_args);
                // TODO: return error?
                Ok(None)
            }
            (_, "identity") => Ok(args.function_args.into_iter().next()),
            (service, function) => {
                let error = f!("service not found: {service} {function}");
                println!("{}", error);
                log::warn!("{}", error);
                Err(JError::new(error))
            }
        };

    Ok(match result {
        Ok(r) => CallServiceResult {
            ret_code: 0,
            result: r.unwrap_or(JValue::String(<_>::default())).to_string(),
        },
        Err(e) => CallServiceResult {
            ret_code: 1,
            result: JValue::from(e).to_string(),
        },
    })
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

pub fn make_particle(
    peer_id: PeerId,
    service_in: &HashMap<String, JValue>,
    script: String,
    relay: impl Into<Option<PeerId>>,
    local_vm: &mut AVM<DataStoreError>,
    generated: bool,
    particle_ttl: Duration,
) -> Result<Particle, Vec<JValue>> {
    let load_variables = if generated {
        "   (null)".to_string()
    } else {
        service_in
            .keys()
            .map(|name| f!(r#"  (call %init_peer_id% ("load" "{name}") [] {name})"#))
            .fold(Instruction::Null, |acc, call| acc.add(call))
            .into_air()
    };

    let catch = f!(r#"(call %init_peer_id% ("errorHandlingSrv" "error") [%last_error%])"#);
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
                &call_results,
            )
            .expect("execute & make particle");

        particle_data = data;
        call_results.clear();

        if call_requests.is_empty() {
            break;
        }
        for (id, call) in call_requests {
            let result = host_call(service_in, call);
            match result {
                Ok(call_result) => {
                    call_results.insert(id, call_result);
                }
                Err(returned_data) => return Err(returned_data),
            }
        }
    }

    log::info!("Made a particle {}", id);

    Ok(Particle {
        id,
        init_peer_id: peer_id,
        timestamp: now_ms() as u64,
        ttl: particle_ttl.as_millis() as u32,
        script,
        signature: vec![],
        data: particle_data,
    })
}

pub fn read_args(
    particle: Particle,
    peer_id: PeerId,
    local_vm: &mut AVM<DataStoreError>,
) -> Vec<JValue> {
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
                &call_results,
            )
            .expect("execute & make particle");

        particle_data = data;
        call_results.clear();

        if call_requests.is_empty() {
            return vec![];
        }
        for (id, call) in call_requests {
            let result = host_call(&<_>::default(), call);
            match result {
                Ok(call_result) => {
                    call_results.insert(id, call_result);
                }
                Err(returned_data) => return returned_data,
            }
        }
    }
}
