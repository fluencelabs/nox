/*
 * Copyright 2019 Fluence Labs Limited
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

// TODO: add some docs

mod gas_costs;

use pwasm_utils::rules;

use clap::{App, AppSettings, Arg, SubCommand};
use exitfailure::ExitFailure;
use failure::err_msg;
use parity_wasm::elements::Module;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

const IN_MODULE_PATH: &str = "in_module_path";
const OUT_MODULE_PATH: &str = "out_module_path";

fn prepare_wasm_file<'a, 'b>() -> App<'a, 'b> {
    let args = &[
        Arg::with_name(IN_MODULE_PATH)
            .required(true)
            .takes_value(true)
            .help("path to the wasm file"),
        Arg::with_name(OUT_MODULE_PATH)
            .required(true)
            .takes_value(true)
            .help("path to a result wasm file"),
    ];

    SubCommand::with_name("prepare")
        .about("Prepare a wasm file to run on the Fluence network")
        .args(args)
}

fn gas_metering<'a, 'b>() -> App<'a, 'b> {
    let args = &[
        Arg::with_name(IN_MODULE_PATH)
            .required(true)
            .takes_value(true)
            .help("path to the wasm file"),
        Arg::with_name(OUT_MODULE_PATH)
            .required(true)
            .takes_value(true)
            .help("path to a result wasm file"),
    ];

    SubCommand::with_name("gas_metering")
        .about("Add gas metering to a wasm file")
        .args(args)
}

fn eic_metering<'a, 'b>() -> App<'a, 'b> {
    let args = &[
        Arg::with_name(IN_MODULE_PATH)
            .required(true)
            .takes_value(true)
            .help("path to the wasm file"),
        Arg::with_name(OUT_MODULE_PATH)
            .required(true)
            .takes_value(true)
            .help("path to a result wasm file"),
    ];

    SubCommand::with_name("eic_metering")
        .about("Add executed instruction counter metering to a wasm file")
        .args(args)
}

pub fn wasm_worker_with<F>(
    in_module_path: &str,
    out_module_path: &str,
    func: F,
) -> Result<(), ExitFailure>
where
    F: FnOnce(Module) -> Module,
{
    let module =
        parity_wasm::deserialize_file(in_module_path).expect("Error while deserializing file");

    let module = func(module);

    parity_wasm::serialize_to_file(out_module_path, module).expect("Error while serializing file");

    Ok(())
}

fn main() -> Result<(), ExitFailure> {
    let app = App::new("Fluence wasm-utils")
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(prepare_wasm_file())
        .subcommand(gas_metering())
        .subcommand(eic_metering());

    match app.get_matches().subcommand() {
        ("prepare", Some(arg)) => {
            wasm_worker_with(
                arg.value_of(IN_MODULE_PATH).unwrap(),
                arg.value_of(OUT_MODULE_PATH).unwrap(),
                |module: Module| {
                    // instrument for gas metering
                    let gas_rules = rules::Set::new(1, gas_costs::gas_cost_table());
                    let module = pwasm_utils::inject_gas_counter(module, &gas_rules)
                        .expect("Error while deserializing file");

                    // instrument for EIC metering
                    let gas_rules = rules::Set::new(1, Default::default());
                    pwasm_utils::inject_gas_counter(module, &gas_rules)
                        .expect("Error while deserializing file")
                },
            )?;
            Ok(())
        }

        ("eic", Some(arg)) => {
            wasm_worker_with(
                arg.value_of(IN_MODULE_PATH).unwrap(),
                arg.value_of(OUT_MODULE_PATH).unwrap(),
                |module: Module| {
                    let gas_rules = rules::Set::new(1, Default::default());
                    pwasm_utils::inject_gas_counter(module, &gas_rules)
                        .expect("Error while deserializing file")
                },
            )?;
            Ok(())
        }

        ("gas", Some(arg)) => {
            wasm_worker_with(
                arg.value_of(IN_MODULE_PATH).unwrap(),
                arg.value_of(OUT_MODULE_PATH).unwrap(),
                |module: Module| {
                    let gas_rules = rules::Set::new(1, gas_costs::gas_cost_table());
                    pwasm_utils::inject_gas_counter(module, &gas_rules)
                        .expect("Error while deserializing file")
                },
            )?;
            Ok(())
        }

        c => Err(err_msg(format!("Unexpected command: {}", c.0)))?,
    }
}
