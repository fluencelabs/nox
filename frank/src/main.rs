/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
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
