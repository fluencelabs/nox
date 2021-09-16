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

use crate::{Args, ArgsError};
use serde_json::Value as JValue;

pub fn from_base58(name: &'static str, args: &mut impl Iterator<Item = JValue>) -> Result<Vec<u8>, ArgsError> {
    let result: String = Args::next(name, args)?;
    bs58::decode(result).into_vec().map_err(|err| ArgsError::InvalidFormat {
        field: "key",
        err: format!("not a base58: {}", err).into(),
    })
}
