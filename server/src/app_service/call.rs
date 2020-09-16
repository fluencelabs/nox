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

use faas_api::FunctionCall;
use fluence_app_service::{CallParameters, IValue};
use serde::{ser::Error as SerError, Serialize, Serializer};
use serde_json::Value;

#[derive(Debug, Clone)]
/// Call to an app service
pub enum ServiceCall {
    /// Call to the app service specified by `service_id`
    Call {
        /// UUID of the app service
        service_id: String,
        /// Module to call function on
        module: String,
        /// Function name to call
        function: String,
        /// Arguments for the function
        // TODO: change to Vec<u8> in future?
        arguments: serde_json::Value,
        /// Call headers
        headers: CallParameters,
        /// FunctionCall that caused this WasmCall, returned to caller as is
        call: FunctionCall,
    },
    /// Request to create new app service with given `module_names`
    Create {
        /// Predefined service_id to create service with
        service_id: Option<String>,
        /// Id of the blueprint to create service from
        blueprint_id: String,
        /// FunctionCall that caused this ServiceCall, returned to caller as is
        call: Option<FunctionCall>,
    },
}

impl ServiceCall {
    pub fn create(blueprint_id: String, call: FunctionCall) -> Self {
        Self::Create {
            blueprint_id,
            call: Some(call),
            service_id: None,
        }
    }

    /// Whether this call is of `Create` type
    pub fn is_create(&self) -> bool {
        matches!(self, ServiceCall::Create { .. })
    }

    pub fn service_id(&self) -> Option<&str> {
        match self {
            ServiceCall::Call { service_id, .. } => Some(service_id),
            ServiceCall::Create { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
/// Result of executing ServiceCall
pub enum ServiceCallResult {
    /// Service was created with this `service_id`
    ServiceCreated { service_id: String },
    #[serde(serialize_with = "ServiceCallResult::serialize_returned")]
    /// Call to a service returned this result
    Returned(Vec<IValue>),
}

impl ServiceCallResult {
    fn serialize_returned<S>(
        value: &[IValue],
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if value.is_empty() {
            Value::Null.serialize(serializer)
        } else {
            let value = fluence_app_service::from_interface_values(&value)
                .map_err(|e| SerError::custom(format!("Failed to serialize result: {}", e)))?;

            Value::serialize(&value, serializer)
        }
    }
}
