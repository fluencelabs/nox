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
use hex::ToHex;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

#[derive(Debug)]
pub struct EthCall<'a, 'b, 'c> {
    data: &'c [u8],
    to: &'a str,
    from: Option<&'b str>,
}

impl<'a, 'b, 'c> EthCall<'a, 'b, 'c> {
    pub fn to(data: &'c [u8], to: &'a str) -> Self {
        Self {
            data,
            to,
            from: None,
        }
    }

    #[allow(unused)]
    pub fn from(data: &'c [u8], to: &'a str, from: &'b str) -> Self {
        Self {
            data,
            to,
            from: Some(from),
        }
    }
}

impl<'a, 'b, 'c> Serialize for EthCall<'a, 'b, 'c> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields = if self.from.is_some() { 3 } else { 2 };
        let mut eth_call = serializer.serialize_struct("EthCall", fields)?;
        eth_call.serialize_field("data", &format!("0x{}", self.data.encode_hex::<String>()))?;
        eth_call.serialize_field("to", self.to)?;
        if let Some(from) = &self.from {
            eth_call.serialize_field("from", from)?;
        }

        eth_call.end()
    }
}

#[cfg(test)]
mod tests {
    use super::EthCall;

    #[test]
    fn serialize_eth_call() {
        let eth_call = EthCall::to(&[0, 1, 2], "0xabc");
        let j = serde_json::json!(eth_call).to_string();
        assert_eq!(j, r#"{"data":"0x000102","to":"0xabc"}"#);

        let eth_call = EthCall::from(&[0, 1, 2], "0xabc", "0xcba");
        let j = serde_json::json!(eth_call).to_string();
        assert_eq!(j, r#"{"data":"0x000102","to":"0xabc","from":"0xcba"}"#);
    }
}
