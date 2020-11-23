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

use crate::args::Args;

use fce::HostImportDescriptor;
use ivalue_utils::{into_record, into_record_opt, IValue};

use serde_json::Value as JValue;
use std::sync::Arc;

pub type Closure = Arc<dyn Fn(Args) -> Option<IValue> + Send + Sync + 'static>;
pub type ClosureDescriptor = Arc<dyn Fn() -> HostImportDescriptor + Send + Sync + 'static>;

/// Converts Fn into Closure, converting error into Option<IValue>
pub fn closure_opt<F>(f: F) -> Closure
where
    F: Fn(std::vec::IntoIter<JValue>) -> Result<Option<JValue>, JValue> + Send + Sync + 'static,
{
    Arc::new(move |Args { args, .. }| into_record_opt(f(args.into_iter())))
}

/// Converts Fn into Closure, converting error into Option<IValue>
pub fn closure<F>(f: F) -> Closure
where
    F: Fn(std::vec::IntoIter<JValue>) -> Result<JValue, JValue> + Send + Sync + 'static,
{
    Arc::new(move |Args { args, .. }| into_record(f(args.into_iter())))
}

/// Converts Fn into Closure, converting error into Option<IValue>
pub fn closure_args<F>(f: F) -> Closure
where
    F: Fn(Args) -> Result<JValue, JValue> + Send + Sync + 'static,
{
    Arc::new(move |args| into_record(f(args)))
}
