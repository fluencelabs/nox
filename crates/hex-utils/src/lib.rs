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

pub use hex::FromHexError;

pub fn decode_hex(h: &str) -> Result<Vec<u8>, FromHexError> {
    let h = h.trim_start_matches("0x");
    hex::decode(h)
}

pub fn encode_hex_0x<T: AsRef<[u8]>>(data: T) -> String {
    let hex = hex::encode(data);
    format!("0x{}", hex)
}

pub fn encode_hex_0x_zero_pad<T: AsRef<[u8]>>(data: T, width: usize) -> String {
    format!("0x{:0>width$}", hex::encode(data))
}

#[cfg(feature = "serde_with")]
pub mod serde_as {
    use core::fmt;
    use hex::{FromHex, ToHex};
    use serde_with::__private__::{DeError, Visitor};
    use serde_with::serde::{Deserializer, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};
    use std::fmt::Display;
    use std::marker::PhantomData;

    pub struct Hex;
    impl<T> SerializeAs<T> for Hex
    where
        T: ToHex,
    {
        fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.collect_str(source.encode_hex::<String>().as_str())
        }
    }

    impl<'de, T> DeserializeAs<'de, T> for Hex
    where
        T: FromHex,
        T::Error: Display,
    {
        fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct Helper<S>(PhantomData<S>);
            impl<'de, S> Visitor<'de> for Helper<S>
            where
                S: FromHex,
                <S as FromHex>::Error: Display,
            {
                type Value = S;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("a string")
                }

                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: DeError,
                {
                    Self::Value::from_hex(value.as_bytes()).map_err(DeError::custom)
                }
            }

            deserializer.deserialize_str(Helper(PhantomData))
        }
    }
}
