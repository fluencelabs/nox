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

use crate::{aquamarine_fname, now, uuid};

use host_closure::Args;
use ivalue_utils::{IType, IValue};
use particle_actors::HostImportDescriptor;
use particle_protocol::Particle;

use aquamarine_vm::{AquamarineVM, AquamarineVMConfig, HostExportedFunc, StepperOutcome};

use fstrings::f;
use libp2p::PeerId;
use parking_lot::Mutex;
use serde_json::Value as JValue;
use std::{collections::HashMap, ops::Deref, sync::Arc};

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction {
    Seq(Box<Instruction>, Box<Instruction>),
    Call(String),
    Null,
}

impl Instruction {
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
            Null => "(null ())".to_string(),
            Call(call) => call,
            Seq(l, r) => {
                let l = l.into_air();
                let r = r.into_air();
                f!("(seq (
{l}
{r}
))")
            }
        }
    }
}

fn route(args: Vec<IValue>, data: HashMap<&'static str, JValue>) -> Option<IValue> {
    let args = Args::parse(args).expect("valid args");
    match args.service_id.as_str() {
        "load" => data
            .get(args.fname.as_str())
            .map(|v| ivalue_utils::ok(v.clone()))
            .unwrap_or(ivalue_utils::error(JValue::String(f!(
                "variable not found: {args.fname}"
            )))),
        "identity" => ivalue_utils::ok(JValue::Array(args.args)),
        service => ivalue_utils::error(JValue::String(f!("service not found: {service}"))),
    }
}

pub fn pass_data_func(data: HashMap<&'static str, JValue>) -> HostExportedFunc {
    Box::new(move |_, args| route(args, data.clone()))
}

pub fn return_data_func(out: Arc<Mutex<Vec<JValue>>>) -> HostExportedFunc {
    Box::new(move |_, args| {
        let args = Args::parse(args).expect("valid args");
        match args.service_id.as_str() {
            "return" => {
                (*out.lock()) = args.args;
                ivalue_utils::unit()
            }
            "identity" => ivalue_utils::ok(JValue::Array(args.args)),
            service => ivalue_utils::error(JValue::String(f!("service not found: {service}"))),
        }
    })
}

fn make_vm(particle_id: String, peer_id: &PeerId, host_func: HostExportedFunc) -> AquamarineVM {
    let call_service = HostImportDescriptor {
        host_exported_func: host_func,
        argument_types: vec![IType::String, IType::String, IType::String],
        output_type: Some(IType::Record(0)),
        error_handler: None,
    };

    let config = AquamarineVMConfig {
        aquamarine_wasm_path: aquamarine_fname(None),
        call_service,
        current_peer_id: peer_id.to_string(),
        particle_data_store: format!("/tmp/{}", particle_id).into(),
    };
    log::info!("particle_data_store: {:?}", config.particle_data_store);

    let vm = AquamarineVM::new(config).expect("vm should be created");

    vm
}

pub fn make_particle(
    peer_id: PeerId,
    data: HashMap<&'static str, JValue>,
    script: String,
) -> Particle {
    let variable_names = data.keys().cloned().collect::<Vec<_>>();

    let load_variables = variable_names
        .into_iter()
        .map(|name| f!(r#"(call ("{peer_id}" ("load" "{name}") () {name}))"#))
        .fold(Instruction::Null, |acc, call| acc.add(call))
        .into_air();
    let script = f!(r#"
(seq (
    {load_variables}
    {script}
))
    "#);

    // log::info!("script\n{}", script);

    let id = uuid();
    let mut vm = make_vm(id.clone(), &peer_id, pass_data_func(data));

    let StepperOutcome { data, .. } = vm
        .call(peer_id.to_string(), script.clone(), "[]", id.clone())
        .expect("execute & make particle");

    log::info!("Made a particle {}", id);

    Particle {
        id,
        init_peer_id: peer_id,
        timestamp: now(),
        ttl: 10000,
        script,
        signature: vec![],
        data: serde_json::from_str(&data).expect("valid json"),
    }
}

pub fn read_args(particle: Particle, peer_id: &PeerId) -> Vec<JValue> {
    let data: Arc<Mutex<Vec<JValue>>> = <_>::default();
    let mut vm = make_vm(
        particle.id.clone(),
        &peer_id,
        return_data_func(data.clone()),
    );
    vm.call(
        peer_id.to_string(),
        particle.script,
        particle.data.to_string(),
        particle.id,
    )
    .expect("execute read_args vm");

    let data = data.lock();
    data.deref().clone()
}
