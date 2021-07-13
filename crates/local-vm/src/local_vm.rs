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

use fs_utils::make_tmp_dir;
use host_closure::Args;
use now_millis::now_ms;
use particle_protocol::Particle;
use uuid_utils::uuid;

use avm_server::{AVMConfig, CallServiceClosure, InterpreterOutcome, AVM};

use air_interpreter_fs::{air_interpreter_path, write_default_air_interpreter};
use fstrings::f;
use libp2p::PeerId;
use parking_lot::Mutex;
use serde_json::Value as JValue;
use std::path::PathBuf;
use std::{collections::HashMap, ops::Deref, sync::Arc, time::Duration};

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

pub fn make_call_service_closure(
    service_in: Arc<Mutex<HashMap<String, JValue>>>,
    service_out: Arc<Mutex<Vec<JValue>>>,
) -> CallServiceClosure {
    Box::new(move |args| {
        let args = Args::parse(args.function_args).expect("valid args");
        match (args.service_id.as_str(), args.function_name.as_str()) {
            ("load", _) | ("getDataSrv", _) => service_in
                .lock()
                .get(args.function_name.as_str())
                .map(|v| ivalue_utils::ok(v.clone()))
                .unwrap_or_else(|| {
                    ivalue_utils::error(JValue::String(f!(
                        "variable not found: {args.function_name}"
                    )))
                }),
            ("return", _) | ("op", "return") | ("callbackSrv", "response") => {
                service_out.lock().extend(args.function_args);
                ivalue_utils::unit()
            }
            ("errorHandlingSrv", "error") => {
                log::warn!("caught an error: {:?}", args.function_args);
                ivalue_utils::unit()
            }
            (_, "identity") => ivalue_utils::ok(JValue::Array(args.function_args)),
            (service, function) => {
                let error = f!("service not found: {service} {function}");
                println!("{}", error);
                log::warn!("{}", error);
                ivalue_utils::error(JValue::String(error))
            }
        }
    })
}

pub fn make_vm(peer_id: PeerId, call_service: CallServiceClosure) -> AVM {
    let tmp = make_tmp_dir();
    let interpreter = air_interpreter_path(&tmp);
    write_default_air_interpreter(&interpreter).expect("write air interpreter");

    let particle_data_store: PathBuf = format!("/tmp/{}", peer_id.to_string()).into();
    let config = AVMConfig {
        call_service,
        air_wasm_path: interpreter,
        current_peer_id: peer_id.to_string(),
        vault_dir: particle_data_store.join("vault"),
        particle_data_store,
        logging_mask: i32::MAX,
    };
    log::info!("particle_data_store: {:?}", config.particle_data_store);

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
    service_in: Arc<Mutex<HashMap<String, JValue>>>,
    script: String,
    relay: impl Into<Option<PeerId>>,
    local_vm: &mut AVM,
    generated: bool,
    particle_ttl: Duration,
) -> Particle {
    let load_variables = if generated {
        "   (null)".to_string()
    } else {
        service_in
            .lock()
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

    let InterpreterOutcome {
        data,
        ret_code,
        error_message,
        ..
    } = local_vm
        .call(peer_id.to_string(), script.clone(), "[]", id.clone())
        .expect("execute & make particle");

    service_in.lock().clear();

    if ret_code != 0 {
        log::error!("failed to make a particle {}: {}", ret_code, error_message);
        panic!("failed to make a particle {}: {}", ret_code, error_message);
    }

    log::info!("Made a particle {}", id);

    Particle {
        id,
        init_peer_id: peer_id,
        timestamp: now_ms() as u64,
        ttl: particle_ttl.as_millis() as u32,
        script,
        signature: vec![],
        data,
    }
}

pub fn read_args(
    particle: Particle,
    peer_id: PeerId,
    local_vm: &mut AVM,
    out: Arc<Mutex<Vec<JValue>>>,
) -> Vec<JValue> {
    let result = local_vm
        .call(
            peer_id.to_string(),
            particle.script,
            particle.data,
            particle.id,
        )
        .expect("execute read_args vm");

    assert_eq!(
        result.ret_code, 0,
        "read_args failed: {}",
        result.error_message
    );

    let result = out.lock().deref().clone();
    out.lock().clear();

    result
}
