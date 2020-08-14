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
use crate::app_service::ServiceExecError;
use faas_api::{Address, FunctionCall};
use fluence_app_service::AppServiceError;
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
            CallErrorKind::ResultSerializationFailed(err_msg) => {
                format!("failed to serialize result: {}", err_msg)
            }
            CallErrorKind::BuiltinServiceError(err) => format!("builtin service failure: {}", err),
            CallErrorKind::AppServiceError(err) => {
                format!("faas service execution failure: {}", err)
            }
            CallErrorKind::Signature(err) => {
                format!("failed to register service, siganture error: {:?}", err)
            }
            CallErrorKind::ServiceRegister(err) => {
                format!("failed to register service, store error: {:?}", err)
            }
            CallErrorKind::NonLocalRelay => {
                "failed to register service, non-local relay".to_string()
            }
            CallErrorKind::UnsupportedProvider(addr) => format!(
                "failed to register service, unsupported provider address {}",
                addr
            ),
            CallErrorKind::MissingReplyTo => "missing reply_to".to_string(),
            CallErrorKind::MissingPublicKey => "can't extract public key from peer id".to_string(),
            CallErrorKind::UnsupportedPublicKey => {
                "unsupported public key, expected ed25519".to_string()
            }
            CallErrorKind::AddCertificates(failed) => {
                format!("failed to add certificates: {:?}", failed)
            }
            CallErrorKind::MissingServiceId => {
                "service id must be specified after # in the target address".to_string()
            }
            CallErrorKind::NoSuchModule { module, service_id } => {
                format!("module {} wasn't found on service {}", module, service_id)
            }
            CallErrorKind::FaaSExecError(err) => {
                format!("error while executing faas call: {}", err)
            }
        }
    }
}

pub enum CallErrorKind {
    MissingFunctionName { module: String },
    FunctionNotFound { module: String, function: String },
    ResultSerializationFailed(String),
    BuiltinServiceError(builtin_service::BuiltinServiceError),
    AppServiceError(AppServiceError),
    Signature(SignatureError),
    ServiceRegister(libp2p::kad::store::Error),
    NonLocalRelay,
    UnsupportedProvider(Address),
    MissingReplyTo,
    MissingPublicKey,
    UnsupportedPublicKey,
    AddCertificates(Vec<(Certificate, String)>),
    MissingServiceId,
    NoSuchModule { module: String, service_id: String },
    FaaSExecError(ServiceExecError),
}

impl From<builtin_service::BuiltinServiceError> for CallErrorKind {
    fn from(err: builtin_service::BuiltinServiceError) -> Self {
        CallErrorKind::BuiltinServiceError(err)
    }
}

impl From<AppServiceError> for CallErrorKind {
    fn from(err: AppServiceError) -> Self {
        CallErrorKind::AppServiceError(err)
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

impl From<ServiceExecError> for CallErrorKind {
    fn from(err: ServiceExecError) -> Self {
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
