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
use serde_json::Value;

#[derive(Debug)]
pub(super) struct WaitPublished {
    pub(super) call: FunctionCall,
    pub(super) reply: Option<Value>,
}

impl From<FunctionCall> for WaitPublished {
    fn from(call: FunctionCall) -> Self {
        WaitPublished { call, reply: None }
    }
}

/// FunctionCall waiting for something happen with Address possible states
#[derive(Debug)]
pub(super) enum WaitAddress {
    /// Waiting until provider for an address is resolved through DHT
    ProviderFound(FunctionCall),
    /// Waiting until provider for an address is published
    Published(WaitPublished),
}

impl WaitAddress {
    pub fn call(self) -> FunctionCall {
        match self {
            WaitAddress::ProviderFound(call) => call,
            WaitAddress::Published(WaitPublished { call, .. }) => call,
        }
    }

    pub fn reply(self) -> (FunctionCall, Option<Value>) {
        match self {
            WaitAddress::ProviderFound(call) => (call, None),
            WaitAddress::Published(WaitPublished { call, reply }) => (call, reply),
        }
    }

    pub fn provider_found(&self) -> bool {
        matches!(self, WaitAddress::ProviderFound(_))
    }

    pub fn published(&self) -> bool {
        matches!(self, WaitAddress::Published(_))
    }
}
