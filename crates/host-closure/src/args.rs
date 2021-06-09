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

use crate::args_error::ArgsError::{self, MissingField, SerdeJson};

use control_macro::ok_get;
use fluence_app_service::SecurityTetraplet;
use ivalue_utils::{as_str, into_string, IValue};

use serde::Deserialize;
use serde_json::Value as JValue;

#[derive(Debug)]
/// Arguments passed by VM to host on call_service
pub struct Args {
    pub service_id: String,
    pub function_name: String,
    pub function_args: Vec<serde_json::Value>,
    pub tetraplets: Vec<Vec<SecurityTetraplet>>,
}

impl Args {
    /// Construct Args from `Vec<IValue>`
    pub fn parse(call_args: Vec<IValue>) -> Result<Args, ArgsError> {
        let mut call_args = call_args.into_iter();
        let service_id = call_args
            .next()
            .and_then(into_string)
            .ok_or(MissingField("service_id"))?;

        let fname = call_args
            .next()
            .and_then(into_string)
            .ok_or(MissingField("fname"))?;

        let function_args = call_args
            .next()
            .as_ref()
            .and_then(as_str)
            .ok_or(MissingField("args"))
            .and_then(|v| {
                serde_json::from_str(v).map_err(|err| SerdeJson { field: "args", err })
            })?;

        let tetraplets: Vec<Vec<SecurityTetraplet>> = call_args
            .next()
            .as_ref()
            .and_then(as_str)
            .ok_or(MissingField("tetraplets"))
            .and_then(|v| {
                serde_json::from_str(v).map_err(|err| SerdeJson {
                    field: "tetraplets",
                    err,
                })
            })?;

        Ok(Args {
            service_id,
            function_name: fname,
            function_args,
            tetraplets,
        })
    }

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

    /// Retrieves a json value from iterator if it's not empty, and parses it to T
    /// `field` is to generate a more accurate error message
    pub fn maybe_next<T: for<'de> Deserialize<'de>>(
        field: &'static str,
        args: &mut impl Iterator<Item = JValue>,
    ) -> Result<Option<T>, ArgsError> {
        let value = ok_get!(args.next());
        let value: T = Self::deserialize(field, value)?;

        Ok(Some(value))
    }

    /// Retrieves a json value from iterator if it's not empty, and parses it to Aqua's option representation
    /// Aqua's option is expected to be an array of 1 or 0 elements.
    /// For the sakes of backward compatibility, scalar value and absence of value are tolerated as well.
    /// `field` is to generate a more accurate error message
    pub fn next_opt<T: for<'de> Deserialize<'de>>(
        field: &'static str,
        args: &mut impl Iterator<Item = JValue>,
    ) -> Result<Option<T>, ArgsError> {
        let value = ok_get!(args.next());
        let value = match value {
            // If there are several values in the passed array,
            // then that could be either a Some(array) in scalar form or an invalid option.
            // The only way to tell one from the other is by looking at the return type.
            // So that's exactly what I do here: asking T to deserialize itself.
            JValue::Array(values) if values.len() > 1 => JValue::Array(values),
            JValue::Array(values) => {
                ok_get!(values.into_iter().next())
            }
            value => value,
        };

        let value: T = Self::deserialize(field, value)?;
        Ok(Some(value))
    }

    /// `field` is to generate a more accurate error message
    fn deserialize<T: for<'de> Deserialize<'de>>(
        field: &'static str,
        v: JValue,
    ) -> Result<T, ArgsError> {
        serde_json::from_value(v).map_err(|err| SerdeJson { err, field })
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
            json!([]),           // as String => None
            json!(["hi"]),       // as String => Some
            json!(["hi", "hi"]), // as Vec    => Some
            json!([]),           // as Vec    => None
            json!(["hi", "hi"]), // as String => Error
            json!("hi"),         // as String => Some
            /* absence */        // as String => None
            /* absence */        // as Vec    => None
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
            "Error while deserializing field scalar: invalid type: sequence, expected a string",
            scalar_err.err().unwrap().to_string()
        );

        let some: Result<Option<String>, _> = Args::next_opt("", &mut args);
        assert_eq!(some.unwrap(), Some(hi()));

        let none: Result<Option<String>, _> = Args::next_opt("", &mut args);
        assert_eq!(none.unwrap(), None);

        let none_vec: Result<Option<Vec<String>>, _> = Args::next_opt("", &mut args);
        assert_eq!(none_vec.unwrap(), None);
    }
}
