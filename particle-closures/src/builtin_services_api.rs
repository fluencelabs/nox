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

use crate::mailbox::{BuiltinCommand, Command, Destination};

use host_closure::{closure, Args, ArgsError, Closure};

use multihash::{Code, MultihashDigest};
use serde_json::Value as JValue;
use std::sync::mpsc as std_mpsc;

#[derive(Debug, Clone)]
pub struct BehaviourMailboxApi {
    mailbox: Destination,
}

impl BehaviourMailboxApi {
    pub fn new(mailbox: Destination) -> Self {
        Self { mailbox }
    }

    pub fn resolve(self) -> Closure {
        closure(move |args| {
            let key = from_base58("key", &mut args.into_iter())?.into();
            self.clone().exec(BuiltinCommand::DHTResolve(key))
        })
    }

    pub fn neighborhood(self) -> Closure {
        closure(move |args| {
            log::warn!("neighborhood args: {:#?}", args);
            let key = from_base58("key", &mut args.into_iter())?;
            log::warn!("neighborhood key from base58: {:?}", key);
            let key = Code::Sha2_256.digest(&key);
            self.clone().exec(BuiltinCommand::DHTNeighborhood(key))
        })
    }

    fn exec(&self, kind: BuiltinCommand) -> Result<JValue, JValue> {
        let (outlet, inlet) = std_mpsc::channel();
        let cmd = Command { outlet, kind };
        self.mailbox
            .unbounded_send(cmd)
            .expect("builtin => mailbox");

        inlet.recv().expect("receive from behaviour mailbox").into()
    }
}

fn from_base58(
    name: &'static str,
    args: &mut impl Iterator<Item = JValue>,
) -> Result<Vec<u8>, ArgsError> {
    let result: String = Args::next(name, args)?;
    bs58::decode(result)
        .into_vec()
        .map_err(|err| ArgsError::InvalidFormat {
            field: "key",
            err: format!("not a base58: {}", err).into(),
        })
}
