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

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::{Borrow, Cow};
use std::fmt::Display;

#[derive(Eq, Clone, Debug, Hash, PartialEq, PartialOrd, Ord)]
pub struct DealId(String);

impl DealId {
    pub fn normalize(str: &str) -> String {
        str.trim_start_matches("0x").to_ascii_lowercase()
    }

    pub fn get_contract_address(&self) -> String {
        format!("0x{}", self.0)
    }

    pub fn to_address(&self) -> String {
        format!("0x{}", self.0)
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        hex::decode(&self.0).unwrap()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for DealId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <Cow<'de, str>>::deserialize(deserializer)?;
        Ok(DealId::from(s.borrow()))
    }
}

impl Serialize for DealId {
    fn serialize<S>(&self, s: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        self.0.to_string().serialize(s)
    }
}

impl PartialEq<&str> for DealId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == DealId::normalize(other)
    }
}

impl PartialEq<&str> for &DealId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == DealId::normalize(other)
    }
}

impl PartialEq<String> for DealId {
    fn eq(&self, other: &String) -> bool {
        self.0 == DealId::normalize(other)
    }
}

impl PartialEq<String> for &DealId {
    fn eq(&self, other: &String) -> bool {
        self.0 == DealId::normalize(other)
    }
}

impl From<DealId> for String {
    fn from(deal_id: DealId) -> Self {
        deal_id.0
    }
}

impl From<String> for DealId {
    fn from(deal_id: String) -> Self {
        DealId(Self::normalize(&deal_id))
    }
}

impl From<&DealId> for String {
    fn from(deal_id: &DealId) -> Self {
        deal_id.0.clone()
    }
}

impl From<&str> for DealId {
    fn from(deal_id: &str) -> Self {
        DealId(Self::normalize(deal_id))
    }
}

impl Display for DealId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::deal_id::DealId;

    #[test]
    fn deal_id() {
        let deal_id_prefix_lowercase = "0x1234567890abcdef";
        let deal_id_prefix_uppercase = "0x1234567890ABCDEF";
        let deal_id_no_prefix_lowercase = "1234567890abcdef";
        let deal_id_no_prefix_uppercase = "1234567890ABCDEF";
        let deal_id_prefix_mixed_case = "0x1234567890AbCdEf";
        let deal_id_no_prefix_mixed_case = "1234567890AbCdEf";

        let deals = vec![
            deal_id_prefix_lowercase,
            deal_id_prefix_uppercase,
            deal_id_no_prefix_lowercase,
            deal_id_no_prefix_uppercase,
            deal_id_prefix_mixed_case,
            deal_id_no_prefix_mixed_case,
        ];

        let deals = deals.into_iter().map(DealId::from).collect::<Vec<_>>();

        assert!(deals.iter().all(|deal| deal == deal_id_prefix_lowercase));
        assert!(deals.iter().all(|deal| deal == deal_id_prefix_uppercase));
        assert!(deals.iter().all(|deal| deal == deal_id_no_prefix_lowercase));
        assert!(deals.iter().all(|deal| deal == deal_id_no_prefix_uppercase));
        assert!(deals.iter().all(|deal| deal == deal_id_prefix_mixed_case));
        assert!(deals
            .iter()
            .all(|deal| deal == deal_id_no_prefix_mixed_case));
    }
}
