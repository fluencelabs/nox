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

use crate::{Args, ArgsError};
use serde_json::Value as JValue;

pub fn from_base58(
    name: &'static str,
    args: &mut impl Iterator<Item = JValue>,
) -> Result<Vec<u8>, ArgsError> {
    let result: String = Args::next(name, args)?;
    bs58::decode(result)
        .into_vec()
        .map_err(|err| ArgsError::InvalidFormat {
            field: "key",
            err: format!("not a base58: {err}").into(),
        })
}
