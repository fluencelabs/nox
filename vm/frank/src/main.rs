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

/// Command-line tool intended to test Frank VM.

mod config;
mod errors;
mod frank;
mod frank_result;
mod jni;
mod modules;

#[macro_use]
extern crate lazy_static;

use crate::config::Config;
use clap::{App, AppSettings, Arg, SubCommand};
use exitfailure::ExitFailure;
use failure::err_msg;
use frank::Frank;

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
            let config = Box::new(Config::default());
            let in_module_path = arg.value_of(IN_MODULE_PATH).unwrap();
            let invoke_arg = arg.value_of(INVOKE_ARG).unwrap();

            let _ = Frank::new(&in_module_path, config)
                .map_err(|err| panic!(format!("{}", err)))
                .and_then(|mut executor| executor.invoke(invoke_arg.as_bytes()))
                .map_err(|err| panic!(format!("{}", err)))
                .map(|result| {
                    let outcome_copy = result.outcome.clone();
                    match String::from_utf8(result.outcome) {
                        Ok(s) => println!("result: {}\nspent gas: {} ", s, result.spent_gas),
                        Err(_) => println!(
                            "result: {:?}\nspent gas: {} ",
                            outcome_copy, result.spent_gas
                        ),
                    }
                });

            Ok(())
        }

        c => Err(err_msg(format!("Unexpected command: {}", c.0)))?,
    }
}
