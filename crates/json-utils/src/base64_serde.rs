/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use base64::{engine::general_purpose::STANDARD as base64, Engine};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[allow(clippy::ptr_arg)] // This is a serializer for Vec<u8>, it is required to be of that type
pub fn serialize<S>(value: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    base64.encode(value).serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let str = String::deserialize(deserializer)?;

    base64
        .decode(str)
        .map_err(|e| serde::de::Error::custom(format!("base64 deserialization failed: {e:?}")))
}
