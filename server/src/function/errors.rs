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

use super::builtin_service;
use crate::function::builtin_service::Error;
use faas_api::{Address, FunctionCall};
use fluence_faas::FaaSError;

pub struct CallError<'a> {
    call: FunctionCall,
    kind: CallErrorKind<'a>,
}
impl<'a> CallError<'a> {
    pub fn error(call: FunctionCall, kind: CallErrorKind<'a>) -> Self {
        Self { call, kind }
    }

    pub fn make<K: Into<CallErrorKind<'a>>>(call: FunctionCall, kind: K) -> Self {
        Self::error(call, kind.into())
    }

    pub fn call(self) -> FunctionCall {
        self.call
    }

    pub fn err_msg(&self) -> String {
        match &self.kind {
            CallErrorKind::MissingFunctionName { module } => {
                format!("missing function name, module was {}", module)
            }
            CallErrorKind::FunctionNotFound { module, function } => {
                format!("function {} not found on module {}", function, module)
            }
            CallErrorKind::InvalidArguments { error } => format!("invalid arguments: {}", error),
            CallErrorKind::ResultSerializationFailed(err_msg) => {
                format!("failed to serialize result: {}", err_msg)
            }
            CallErrorKind::BuiltinServiceError(err) => format!("builtin service failure: {}", err),
            CallErrorKind::FaaSError(err) => format!("faas execution failure: {}", err),
            CallErrorKind::UnroutableCall(err_msg) => format!("unroutable call: {}", err_msg),
        }
    }

    #[allow(dead_code)]
    pub fn into_reply(mut self, sender: Address) -> FunctionCall {
        use serde_json::json;

        let err_msg = self.err_msg();
        let arguments = json!({ "reason": err_msg, "call": self.call });
        let reply_to = self.call.reply_to.take().unwrap_or(self.call.sender);

        FunctionCall {
            uuid: format!("error_{}", self.call.uuid),
            target: Some(reply_to),
            reply_to: None,
            module: None,
            fname: None,
            arguments,
            name: self.call.name,
            sender,
        }
    }
}

pub enum CallErrorKind<'a> {
    MissingFunctionName { module: String },
    FunctionNotFound { module: String, function: String },
    InvalidArguments { error: String },
    ResultSerializationFailed(String),
    BuiltinServiceError(builtin_service::Error<'a>),
    FaaSError(FaaSError),
    UnroutableCall(String),
}

impl<'a> CallErrorKind<'a> {
    #[allow(dead_code)]
    pub fn of_call(self, call: FunctionCall) -> CallError<'a> {
        CallError::make(call, self)
    }
}

impl<'a> From<builtin_service::Error<'a>> for CallErrorKind<'a> {
    fn from(err: Error<'a>) -> Self {
        CallErrorKind::BuiltinServiceError(err)
    }
}

impl From<FaaSError> for CallErrorKind<'static> {
    fn from(err: FaaSError) -> Self {
        CallErrorKind::FaaSError(err)
    }
}
