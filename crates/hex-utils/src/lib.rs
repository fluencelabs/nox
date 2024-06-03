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

pub fn decode_hex(h: &str) -> Result<Vec<u8>, hex::FromHexError> {
    let h = h.trim_start_matches("0x");
    hex::decode(h)
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
