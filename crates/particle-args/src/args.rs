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

use crate::args_error::ArgsError::{self, MissingField};

use control_macro::ok_get;

use crate::ArgsError::DeserializeError;
use avm_server::{CallRequestParams, SecurityTetraplet};
use serde::Deserialize;
use serde_json::Value as JValue;
use std::convert::TryFrom;

#[derive(Debug, Clone)]
/// Arguments passed by VM to host on call_service
pub struct Args {
    pub service_id: String,
    pub function_name: String,
    pub function_args: Vec<serde_json::Value>,
    pub tetraplets: Vec<Vec<SecurityTetraplet>>,
}

impl Args {
    /// Retrieves next json value from iterator, parse it to T
    /// `field` is to generate a more accurate error message
    pub fn next<T: for<'de> Deserialize<'de>>(
        field: &'static str,
        args: &mut impl Iterator<Item = JValue>,
    ) -> Result<T, ArgsError> {
        let value = args.next().ok_or(MissingField(field))?;
        let value: T = Self::deserialize(field, value)?;

        Ok(value)
    }

    /// Retrieves a json value from iterator if it's not empty, and parses it to Aqua's option representation
    /// Aqua's option is expected to be an array of 1 or 0 elements.
    /// For the sakes of backward compatibility, scalar value and absence of value are tolerated as well.
    /// `field` is to generate a more accurate error message
    ///
    /// In short, function returns:
    /// - if next arg is `T` or `[T]`               => Some(T)
    /// - if next arg is `None` or `[]`             => None
    /// - if next arg is array of several elements  => error
    pub fn next_opt<T: for<'de> Deserialize<'de>>(
        field: &'static str,
        args: &mut impl Iterator<Item = JValue>,
    ) -> Result<Option<T>, ArgsError> {
        let value = ok_get!(args.next());

        // TODO: get rid of untagged enum, and always deserialize to Vec<T> of one element.
        #[derive(serde::Deserialize, Debug)]
        #[serde(untagged)]
        pub enum Opt<T> {
            Array(Vec<T>),
            Scalar(T),
            None,
        }
        let value: Opt<T> = Self::deserialize(field, value)?;
        let value = match value {
            Opt::Scalar(v) => Some(v),
            Opt::Array(v) if v.len() > 1 => {
                return Err(ArgsError::NonUnaryOption {
                    field,
                    length: v.len(),
                })
            }
            Opt::Array(v) => v.into_iter().next(),
            Opt::None => None,
        };
        Ok(value)
    }

    /// `field` is to generate a more accurate error message
    fn deserialize<T: for<'de> Deserialize<'de>>(
        field: &'static str,
        value: JValue,
    ) -> Result<T, ArgsError> {
        serde_json::from_value(value.clone()).map_err(|_| {
            let expected_type = std::any::type_name::<T>();
            let expected_type = expected_type.replace("alloc::vec::", "");
            let expected_type = expected_type.replace("alloc::string::", "");
            let expected_type =
                expected_type.replace("particle_args::args::Args::next_opt::Opt", "Option");

            DeserializeError {
                field,
                value,
                expected_type,
            }
        })
    }
}

impl TryFrom<CallRequestParams> for Args {
    type Error = ArgsError;

    fn try_from(value: CallRequestParams) -> Result<Self, Self::Error> {
        Ok(Args {
            service_id: value.service_id,
            function_name: value.function_name,
            function_args: value.arguments,
            tetraplets: value.tetraplets,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Args;
    use serde_json::json;

    #[test]
    fn test_next_opt() {
        #[rustfmt::skip]
        let mut args = vec![
            json!([]),                    // as String => None
            json!(["hi"]),                // as String => Some
            json!(["hi", "hi"]),          // as Vec    => Some
            json!([]),                    // as Vec    => None
            json!(["hi", "hi"]),          // as String => Error
            json!("hi"),                  // as String => Some
            json!(["hi"]),                // as Vec    => Some
            json!([[[[ "hi", "bye" ]]]])  // as String => Error
            /* absence */                 // as String => None
            /* absence */                 // as Vec    => None
        ]
        .into_iter();

        fn hi() -> String {
            "hi".to_string()
        }

        let none: Result<Option<String>, _> = Args::next_opt("", &mut args);
        assert_eq!(none.unwrap(), None);

        let some: Result<Option<String>, _> = Args::next_opt("", &mut args);
        assert_eq!(some.unwrap(), Some(hi()));

        let some_vec: Result<Option<Vec<String>>, _> = Args::next_opt("", &mut args);
        assert_eq!(some_vec.unwrap(), Some(vec![hi(), hi()]));

        let none_vec: Result<Option<Vec<String>>, _> = Args::next_opt("", &mut args);
        assert_eq!(none_vec.unwrap(), None);

        let scalar_err: Result<Option<String>, _> = Args::next_opt("scalar", &mut args);
        assert!(scalar_err.is_err());
        assert_eq!(
            "Option's array must contain at most 1 element, 'scalar' was of 2 elements",
            scalar_err.err().unwrap().to_string()
        );

        let some: Result<Option<String>, _> = Args::next_opt("", &mut args);
        assert_eq!(some.unwrap(), Some(hi()));

        let some_vec: Result<Option<Vec<String>>, _> = Args::next_opt("", &mut args);
        assert_eq!(some_vec.unwrap(), Some(vec![hi()]));

        let incorrect_type: Result<Option<Vec<(String, String)>>, _> =
            Args::next_opt("field", &mut args);
        assert_eq!(
            r#"Error deserializing field 'field': expected Option<Vec<(String, String)>>, got [[[["hi","bye"]]]]"#,
            incorrect_type.err().unwrap().to_string()
        );

        let none: Result<Option<String>, _> = Args::next_opt("", &mut args);
        assert_eq!(none.unwrap(), None);

        let none_vec: Result<Option<Vec<String>>, _> = Args::next_opt("", &mut args);
        assert_eq!(none_vec.unwrap(), None);
    }
}
