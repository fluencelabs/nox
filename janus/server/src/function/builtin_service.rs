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

use faas_api::{
    service, Address,
    Protocol::{self, *},
};
use janus_libp2p::peerid_serializer;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BuiltinService {
    DelegateProviding {
        service_id: String,
    },
    GetCertificates {
        #[serde(with = "peerid_serializer")]
        peer_id: PeerId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        msg_id: Option<String>,
    },
}

impl BuiltinService {
    const PROVIDE: &'static str = "provide";
    const CERTS: &'static str = "certificates";

    // TODO: maybe implement as From<(Address, json::Value)>?
    pub fn from(target: &Address, arguments: serde_json::Value) -> Option<Self> {
        match target.protocols().as_slice() {
            [Service(service_id)] if service_id == Self::PROVIDE => {
                // expects "service_id" field
                serde_json::from_value(arguments).ok()
            }
            [Service(service_id)] if service_id == Self::CERTS => {
                // expects "peer_id" & msg_id fields
                serde_json::from_value(arguments).ok()
            }
            _ => None,
        }
    }

    pub fn is_builtin(proto: &Protocol) -> bool {
        match proto {
            Service(service_id) => service_id == Self::PROVIDE || service_id == Self::CERTS,
            _ => false,
        }
    }
}

impl Into<(Address, serde_json::Value)> for BuiltinService {
    fn into(self) -> (Address, serde_json::Value) {
        use serde_json::json;

        match self {
            BuiltinService::DelegateProviding { service_id } => {
                let address = service!(BuiltinService::PROVIDE);
                let arguments = json!({ "service_id": service_id });
                (address, arguments)
            }
            BuiltinService::GetCertificates { .. } => {
                let address = service!(BuiltinService::CERTS);
                let arguments = json!(self);
                (address, arguments)
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::BuiltinService;
    use faas_api::call_test_utils::gen_provide_call;
    use faas_api::Protocol;

    #[test]
    fn serialize() {
        let ipfs_service = "IPFS.get_QmFile";
        let (target, arguments) = BuiltinService::DelegateProviding {
            service_id: ipfs_service.into(),
        }
        .into();
        let call = gen_provide_call(target, arguments);

        let target = call.target.as_ref().expect("target should be Some");
        let protocols = target.protocols();

        let service_id = match protocols.as_slice() {
            [Protocol::Service(service_id)] => service_id,
            wrong => unreachable!("target should be Address::Service, was {:?}", wrong),
        };

        assert_eq!(service_id, "provide");

        match BuiltinService::from(target, call.arguments) {
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
