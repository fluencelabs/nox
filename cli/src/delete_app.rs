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

extern crate clap;

use std::boxed::Box;
use std::error::Error;

use contract_func::contract::functions::delete_app;

use clap::{App, Arg, SubCommand};
use clap::ArgMatches;

use utils;

const APP_ID: &str = "app_id";
const CLUSTER_ID: &str = "cluster_id";

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("delete_app")
        .about("Delete app from smart-contract")
        .args(&[
            Arg::with_name(APP_ID)
                .required(true)
                .takes_value(true)
                .help("app to be removed"),
            Arg::with_name(CLUSTER_ID)
                .required(false)
                .takes_value(true)
                .help("ID of cluster hosting the app")
        ])
}

pub fn parse(args: &ArgMatches) -> (H256, Option<H256>) {
    let app_id: H256 = args.value_of(APP_ID).unwrap();
    let cluster_id = args.value_of(CLUSTER_ID);

    return (app_id, cluster_id);
}

pub fn delete_app(app_id: &str, cluster_id: Option<&str>, show_progress: bool) {
    let delete_app_fn = || -> Result<H256, Box<Error>> {
        let (call_data, _) = delete_app::call(app_id, cluster_id.unwrap_or);
    }
}