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

use crate::mailbox::{BuiltinCommand, BuiltinCommandResult, Command, Destination, WaitResult};

use host_closure::{Args, ArgsError, Closure};
use ivalue_utils::into_record;
use json_utils::err_as_value;

use libp2p::kad::record;
use std::{sync::mpsc as std_mpsc, sync::Arc};

#[derive(Debug, Clone)]
pub struct BuiltinServicesApi {
    mailbox: Destination,
}

impl BuiltinServicesApi {
    const SERVICES: &'static [&'static str] = &["services"];

    pub fn new(mailbox: Destination) -> Self {
        Self { mailbox }
    }

    pub fn is_builtin(service_id: &str) -> bool {
        Self::SERVICES.contains(&service_id)
    }

    pub fn router(self) -> Closure {
        Arc::new(move |args| {
            let result = Self::route(self.clone(), args)
                .map_err(err_as_value)
                .and_then(Into::into);
            into_record(result)
        })
    }

    fn route(api: BuiltinServicesApi, args: Args) -> Result<BuiltinCommandResult, ArgsError> {
        let wait = match args.service_id.as_str() {
            "resolve" => {
                let key: String = Args::next("key", &mut args.args.into_iter())?;
                let key = bs58::decode(key)
                    .into_vec()
                    .map_err(|err| ArgsError::InvalidFormat {
                        field: "key",
                        err: format!("not a base58: {:?}", err).into(),
                    })?;

                api.resolve(key.into())
            }
            "add_certificate" => unimplemented!("FIXME"),
            _ => unimplemented!("FIXME: unknown. return error? re-route to call service?"),
        };

        Ok(wait.recv().expect("receive BuiltinCommandResult"))
    }

    fn resolve(&self, key: record::Key) -> WaitResult {
        let (outlet, inlet) = std_mpsc::channel();
        let cmd = Command {
            outlet,
            kind: BuiltinCommand::DHTResolve(key),
        };
        self.mailbox
            .unbounded_send(cmd)
            .expect("builtin => mailbox");

        inlet
    }
}
