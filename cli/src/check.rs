/*
 * Copyright 2018 Fluence Labs Limited
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

use self::errors::*;
use clap::{App, Arg, ArgMatches, SubCommand};
use console::style;
use parity_wasm::elements::Error as ParityError;
use parity_wasm::elements::Module;
use std::collections::HashMap;

const INPUT_ARG: &str = "input";

/// Performs validations according to the specified arguments. Writes results
/// to 'stdout' until some error occurred.
///
/// For now, is only implemented searching for using functions from banned modules.
pub fn process(args: &ArgMatches) -> Result<()> {
    // defines banned modules (todo add more)
    let banned_modules = vec![
        "std::sys::wasm::thread::",
        "std::sys::wasm::fs::",
        "std::sys::wasm::time::",
    ];

    let file = args.value_of(INPUT_ARG).unwrap(); // always defined
    let module = read_wasm_file(file)?;

    //
    // Step 1. Find all functions from specified banned modules
    //

    let banned_functions = find_banned_fns_idxs(&module, &banned_modules)?;

    if banned_functions.is_empty() {
        let msg = format!(
            "\nAll right! There are no functions from banned modules {:?}\n",
            banned_modules
        );
        println!("{}", style(msg).bold());
    } else {
        println!("There was found next banned functions ->");
        banned_functions
            .iter()
            .for_each(|(idx, name)| println!("  idx: {:?} name: {:?}", idx, name));
        println!("For the next banned modules {:?}", banned_modules);
    }

    //
    // Step 2. Build call graph for found functions from banned modules
    //

    // todo finish

    Ok(())
}

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("check")
        .about("Verifies wasm file, issue warning for using functions from banned modules.")
        .args(&[Arg::with_name(INPUT_ARG)
            .required(true)
            .takes_value(true)
            .help("Wasm file for validation")])
}

//
// Private methods
//

/// Reads specified file, parses and returns module representation.
fn read_wasm_file(file: &str) -> Result<Module> {
    parity_wasm::deserialize_file(file)
        .and_then(|module| module.parse_names().map_err(|errors| get_first(&errors)))
        .map_err(Into::into)
}

/// Takes first error from Error list.
fn get_first(errors: &(Vec<(usize, ParityError)>, Module)) -> ParityError {
    errors.0[0].1.clone()
}

/// Finds in the name section all functions from specified banned modules and
/// returns their names and indexes.
fn find_banned_fns_idxs<'a>(
    module: &'a Module,
    banned_modules: &[&str],
) -> Result<HashMap<u32, &'a str>> {
    use parity_wasm::elements::NameSection;

    // name section is actually Map<fn_idx, fn_name> for each function in a module
    module
        .names_section()
        .ok_or_else(|| Error::from("Name section is absent, verification is aborted."))
        .and_then(|name_sec| {
            if let NameSection::Function(fn_name_sec) = name_sec {
                let banned_fns: HashMap<u32, &'a str> = fn_name_sec
                    .names()
                    .iter()
                    .filter_map(|(idx, name)| {
                        banned_modules
                            .iter()
                            .find(|m| name.starts_with(*m))
                            .map(|_| (idx, name.as_str()))
                    })
                    .collect();
                Ok(banned_fns)
            } else {
                Err("Name section for functions is absent, verification is aborted.".into())
            }
        })
}

mod errors {
    error_chain! {
         foreign_links {
            ParityErr(parity_wasm::elements::Error);
        }
    }
}
