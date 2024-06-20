/*
 * Copyright 2024 Fluence DAO
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

use alloy_primitives::{BlockNumber, U256};
use serde_json::Value;
use std::str::FromStr;

#[derive(Debug)]
pub struct BlockHeader {
    pub number: BlockNumber,
    pub timestamp: U256,
}

impl BlockHeader {
    pub fn from_json(json: Value) -> eyre::Result<Self> {
        let obj = json
            .as_object()
            .ok_or(eyre::eyre!("header is not an object; got {json}"))?;

        let timestamp = obj
            .get("timestamp")
            .and_then(Value::as_str)
            .ok_or(eyre::eyre!(" timestamp field not found; got {json}"))?
            .to_string();

        let block_number = obj
            .get("number")
            .and_then(Value::as_str)
            .ok_or(eyre::eyre!("number field not found; got {json}"))?
            .to_string();

        Ok(Self {
            number: BlockNumber::from_str_radix(block_number.trim_start_matches("0x"), 16)?,
            timestamp: U256::from_str(&timestamp)?,
        })
    }

    pub fn from_json_str(json: &str) -> eyre::Result<Self> {
        let json: Value = serde_json::from_str(json)
            .map_err(|err| eyre::eyre!("failed to parse header {err}; got {json}"))?;
        Self::from_json(json)
    }
}
