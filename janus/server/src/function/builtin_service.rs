/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
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
