/*
 * Copyright 2019 Fluence Labs Limited
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

use faas_api::Address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BuiltinService {
    DelegateProviding { service_id: String },
}

impl BuiltinService {
    const PROVIDE: &'static str = "provide";

    // TODO: maybe implement as From<(Address, json::Value)>?
    pub fn from(service_id: &str, arguments: serde_json::Value) -> Option<Self> {
        match service_id {
            Self::PROVIDE => serde_json::from_value(arguments).ok(),
            _ => None,
        }
    }
}

impl Into<(Address, serde_json::Value)> for BuiltinService {
    fn into(self) -> (Address, serde_json::Value) {
        use serde_json::json;

        match self {
            BuiltinService::DelegateProviding { service_id } => {
                let address = Address::Service {
                    service_id: BuiltinService::PROVIDE.into(),
                };
                let arguments = json!({ "service_id": service_id });
                (address, arguments)
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::BuiltinService;
    use faas_api::call_test_utils::gen_provide_call;
    use faas_api::Address;

    #[test]
    fn serialize() {
        let ipfs_service = "IPFS.get_QmFile";
        let (target, arguments) = BuiltinService::DelegateProviding {
            service_id: ipfs_service.into(),
        }
        .into();
        let call = gen_provide_call(target, arguments);

        let service_id = match call.target.clone() {
            Some(Address::Service { service_id }) => service_id,
            wrong => unreachable!("target should be Some(Address::Service), was {:?}", wrong),
        };

        match BuiltinService::from(&service_id, call.arguments) {
            Some(BuiltinService::DelegateProviding { service_id }) => {
                assert_eq!(service_id, ipfs_service)
            }
            wrong => unreachable!(
                "target should be Some(BuiltinService::DelegateProviding, was {:?}",
                wrong
            ),
        };
    }
}
