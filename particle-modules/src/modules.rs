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

use crate::files;

use host_closure::{Args, ArgsError, Closure};
use ivalue_utils::IValue;
use ArgsError::{MissingField, SerdeJson};

use serde::de::DeserializeOwned;
use std::{path::PathBuf, sync::Arc};

#[allow(dead_code)]
pub fn add_module(modules_dir: PathBuf) -> Closure {
    Arc::new(move |Args { mut args, .. }| {
        let mut add_module = || {
            let module = get("module", &mut args).map_err(as_ivalue)?;
            let config = get("config", &mut args).map_err(as_ivalue)?;
            files::add_module(&modules_dir, module, config).map_err(as_ivalue)
        };

        if let Err(err) = add_module() {
            return ivalue_utils::error(err);
        }

        ivalue_utils::unit()
    })
}

pub fn add_blueprint(blueprint_dir: PathBuf) -> Closure {
    Arc::new(move |Args { mut args, .. }| {
        let mut add_blueprint = || {
            let blueprint = get("blueprint", &mut args).map_err(as_ivalue)?;
            files::add_blueprint(&blueprint_dir, &blueprint).map_err(as_ivalue)
        };

        if let Err(err) = add_blueprint() {
            return ivalue_utils::error(err);
        }

        ivalue_utils::unit()
    })
}

fn closure<F>(f: F) -> Closure
where
    F: Fn(serde_json::Value) -> Result<(), IValue> + Send + Sync + 'static,
{
    Arc::new(move |Args { args, .. }| {
        if let Err(err) = f(args) {
            return ivalue_utils::error(err);
        }

        ivalue_utils::unit()
    })
}

fn as_ivalue<E: ToString>(err: E) -> IValue {
    IValue::String(err.to_string())
}

fn get<T: DeserializeOwned>(
    field: &'static str,
    args: &mut serde_json::Value,
) -> Result<T, ArgsError> {
    let value = args.get_mut(field).ok_or(MissingField(field))?.take();
    let value: T = serde_json::from_value(value).map_err(|err| SerdeJson { err, field })?;

    Ok(value)
}

/*fn as_bytes(value: serde_json::Value) -> Option<Vec<u8>> {
    match value {
        serde_json::Value::Array(values) => {
            let bytes: Vec<_> = values
                .into_iter()
                .flat_map(|u| u.as_u64())
                .map(|u| u as u8)
                .collect();
            Some(bytes)
        }
        _ => None,
    }
}*/
