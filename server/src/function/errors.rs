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

use super::{address_signature::SignatureError, builtin_service};
use crate::faas::FaaSExecError;
use faas_api::{Address, FunctionCall};
use fluence_faas::FaaSError;
use trust_graph::Certificate;

pub struct CallError {
    call: FunctionCall,
    kind: CallErrorKind,
}
impl CallError {
    pub fn error(call: FunctionCall, kind: CallErrorKind) -> Self {
        Self { call, kind }
    }

    pub fn make<K: Into<CallErrorKind>>(call: FunctionCall, kind: K) -> Self {
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
            CallErrorKind::Signature(err) => {
                format!("failed to register service, siganture error: {:?}", err)
            }
            CallErrorKind::ServiceRegister(err) => {
                format!("failed to register service, store error: {:?}", err)
            }
            CallErrorKind::NonLocalRelay => {
                "failed to register service, non-local relay".to_string()
            }
            CallErrorKind::UnsupportedReplyTo(addr) => {
                format!("failed to register service, unsupported reply_to {}", addr)
            }
            CallErrorKind::MissingReplyTo => "missing reply_to".to_string(),
            CallErrorKind::MissingPublicKey => "can't extract public key from peer id".to_string(),
            CallErrorKind::UnsupportedPublicKey => {
                "unsupported public key, expected ed25519".to_string()
            }
            CallErrorKind::AddCertificates(failed) => {
                format!("failed to add certificates: {:?}", failed)
            }
            CallErrorKind::FaasInterfaceSerialization(err) => format!(
                "Totally unexpected: can't serialize FaaS interface to json: {}",
                err
            ),
            CallErrorKind::MissingServiceId => {
                format!("service id must be specified after # in the target address")
            }
            CallErrorKind::NoSuchModule { module, service_id } => {
                format!("module {} wasn't found on service {}", module, service_id)
            }
            CallErrorKind::FaaSExecError(err) => {
                format!("error while executing faas call: {}", err)
            }
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
            context: vec![],
        }
    }
}

pub enum CallErrorKind {
    MissingFunctionName {
        module: String,
    },
    FunctionNotFound {
        module: String,
        function: String,
    },
    InvalidArguments {
        error: String,
    },
    ResultSerializationFailed(String),
    BuiltinServiceError(builtin_service::Error),
    FaaSError(FaaSError),
    #[allow(dead_code)]
    UnroutableCall(String),
    Signature(SignatureError),
    ServiceRegister(libp2p::kad::store::Error),
    NonLocalRelay,
    UnsupportedReplyTo(Address),
    MissingReplyTo,
    MissingPublicKey,
    UnsupportedPublicKey,
    AddCertificates(Vec<(Certificate, String)>),
    #[allow(dead_code)]
    FaasInterfaceSerialization(serde_json::Error),
    MissingServiceId,
    NoSuchModule {
        module: String,
        service_id: String,
    },
    FaaSExecError(FaaSExecError),
}

impl CallErrorKind {
    #[allow(dead_code)]
    pub fn of_call(self, call: FunctionCall) -> CallError {
        CallError::make(call, self)
    }
}

impl From<builtin_service::Error> for CallErrorKind {
    fn from(err: builtin_service::Error) -> Self {
        CallErrorKind::BuiltinServiceError(err)
    }
}

impl From<FaaSError> for CallErrorKind {
    fn from(err: FaaSError) -> Self {
        CallErrorKind::FaaSError(err)
    }
}

impl From<SignatureError> for CallErrorKind {
    fn from(err: SignatureError) -> Self {
        CallErrorKind::Signature(err)
    }
}

impl From<libp2p::kad::record::store::Error> for CallErrorKind {
    fn from(err: libp2p::kad::record::store::Error) -> Self {
        CallErrorKind::ServiceRegister(err)
    }
}

impl From<FaaSExecError> for CallErrorKind {
    fn from(err: FaaSExecError) -> Self {
        CallErrorKind::FaaSExecError(err)
    }
}

pub trait ErrorData<EKind, Error> {
    fn error(self, e: EKind) -> Error;
}

impl<'a, E: Into<CallErrorKind>> ErrorData<E, CallError> for FunctionCall {
    fn error(self, e: E) -> CallError {
        CallError::make(self, e)
    }
}
