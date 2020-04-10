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

#![deny(
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

/// Command-line tool intended to test Frank VM.
mod vm;

use crate::vm::config::Config;
use crate::vm::frank::Frank;
use crate::vm::prepare::prepare_module;

use crate::vm::service::FluenceService;
use clap::{App, AppSettings, Arg, SubCommand};
use exitfailure::ExitFailure;
use failure::err_msg;
use std::fs;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

const IN_MODULE_PATH: &str = "in-wasm-path";
const INVOKE_ARG: &str = "arg";

fn prepare_args<'a, 'b>() -> [Arg<'a, 'b>; 2] {
    [
        Arg::with_name(IN_MODULE_PATH)
            .required(true)
            .takes_value(true)
            .short("i")
            .help("path to the wasm file"),
        Arg::with_name(INVOKE_ARG)
            .required(true)
            .takes_value(true)
            .short("a")
            .help("argument for the invoke function in the Wasm module"),
    ]
}

fn execute_wasm<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("execute")
        .about("Execute provided module on the Fluence Frank VM")
        .args(&prepare_args())
}

fn main() -> Result<(), ExitFailure> {
    let app = App::new("Fluence Frank Wasm execution environment for test purposes")
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(execute_wasm());

    match app.get_matches().subcommand() {
        ("execute", Some(arg)) => {
            let config = Config::default();
            let in_module_path = arg.value_of(IN_MODULE_PATH).unwrap();
            let wasm_code = fs::read(in_module_path)?;
            let wasm_code = prepare_module(&wasm_code, &config)?;

            let invoke_arg = arg.value_of(INVOKE_ARG).unwrap();
            let mut frank = Frank::new(&wasm_code, config)?;
            let result = frank.invoke(invoke_arg.as_bytes())?;

            let outcome_copy = result.outcome.clone();
            match String::from_utf8(result.outcome) {
                Ok(s) => println!("result: {}", s),
                Err(_) => println!("result: {:?}", outcome_copy),
            }

            Ok(())
        }

        c => Err(err_msg(format!("Unexpected command: {}", c.0)).into()),
    }
}
