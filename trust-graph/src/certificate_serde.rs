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

use crate::Certificate;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

mod single {
    #![allow(dead_code)]
    use super::*;
    pub fn serialize<S>(value: &Certificate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.to_string().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Certificate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str = String::deserialize(deserializer)?;
        Certificate::from_str(&str)
            .map_err(|e| Error::custom(format!("certificate deserialization failed for {:?}", e)))
    }
}

pub mod vec {
    use super::*;
    use serde::ser::SerializeSeq;

    pub fn serialize<S>(value: &[Certificate], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(value.len()))?;
        for e in value {
            let e = e.to_string();
            seq.serialize_element(&e)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Certificate>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: Vec<String> = Vec::deserialize(deserializer)?;
        v.into_iter()
            .map(|e| {
                Certificate::from_str(&e).map_err(|e| {
                    Error::custom(format!("certificate deserialization failed for {:?}", e))
                })
            })
            .collect()
    }
}
