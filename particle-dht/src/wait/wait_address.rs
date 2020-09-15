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

use serde_json::Value;

#[derive(Debug)]
pub(crate) struct WaitPublished<T> {
    pub(crate) value: T,
    pub(crate) reply: Option<Value>,
}

impl<T> From<T> for WaitPublished<T> {
    fn from(value: T) -> Self {
        WaitPublished { value, reply: None }
    }
}

/// FunctionCall waiting for something happen with Address possible states
#[derive(Debug)]
pub(crate) enum WaitAddress<T> {
    /// Waiting until provider for an address is resolved through DHT
    ProviderFound(T),
    /// Waiting until provider for an address is published
    Published(WaitPublished<T>),
}

impl<T> WaitAddress<T> {
    pub fn value(self) -> T {
        match self {
            WaitAddress::ProviderFound(value) => value,
            WaitAddress::Published(WaitPublished { value, .. }) => value,
        }
    }

    pub fn reply(self) -> (T, Option<Value>) {
        match self {
            WaitAddress::ProviderFound(value) => (value, None),
            WaitAddress::Published(WaitPublished { value, reply }) => (value, reply),
        }
    }

    pub fn provider_found(&self) -> bool {
        matches!(self, WaitAddress::ProviderFound(_))
    }

    pub fn published(&self) -> bool {
        matches!(self, WaitAddress::Published(_))
    }
}
