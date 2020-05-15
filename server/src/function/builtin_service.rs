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

use faas_api::{
    service, Address,
    Protocol::{self, *},
};
use fluence_libp2p::peerid_serializer;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use trust_graph::{certificate_serde, Certificate};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateProviding {
    pub service_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCertificates {
    #[serde(with = "peerid_serializer")]
    pub peer_id: PeerId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddCertificates {
    #[serde(with = "certificate_serde::vec")]
    pub certificates: Vec<Certificate>,
    #[serde(with = "peerid_serializer")]
    pub peer_id: PeerId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BuiltinService {
    DelegateProviding(DelegateProviding),
    GetCertificates(GetCertificates),
    AddCertificates(AddCertificates),
}

impl BuiltinService {
    const PROVIDE: &'static str = "provide";
    const CERTS: &'static str = "certificates";
    const ADD_CERTS: &'static str = "add_certificates";
    const SERVICES: [&'static str; 3] = [Self::PROVIDE, Self::CERTS, Self::ADD_CERTS];

    pub fn from(target: &Address, arguments: serde_json::Value) -> Option<Self> {
        // Check it's `/service/ID` and `ID` is one of builtin services
        let protocols = target.protocols();
        let service_id = match protocols.as_slice() {
            [Service(service_id)] => service_id,
            _ => return None,
        };

        let service = match service_id.as_str() {
            Self::PROVIDE => {
                BuiltinService::DelegateProviding(serde_json::from_value(arguments).ok()?)
            }
            Self::CERTS => BuiltinService::GetCertificates(serde_json::from_value(arguments).ok()?),
            Self::ADD_CERTS => {
                BuiltinService::AddCertificates(serde_json::from_value(arguments).ok()?)
            }
            _ => return None,
        };

        Some(service)
    }

    pub fn is_builtin(proto: &Protocol) -> bool {
        match proto {
            Service(service_id) => Self::SERVICES.contains(&service_id.as_str()),
            _ => false,
        }
    }
}

impl Into<(Address, serde_json::Value)> for BuiltinService {
    fn into(self) -> (Address, serde_json::Value) {
        use serde_json::json;

        let service_id = match self {
            BuiltinService::DelegateProviding { .. } => BuiltinService::PROVIDE,
            BuiltinService::GetCertificates { .. } => BuiltinService::CERTS,
            BuiltinService::AddCertificates { .. } => BuiltinService::ADD_CERTS,
        };
        let address = service!(service_id);
        let arguments = json!(self);

        (address, arguments)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use faas_api::call_test_utils::gen_provide_call;
    use faas_api::Protocol;
    use fluence_libp2p::RandomPeerId;
    use std::str::FromStr;

    #[test]
    fn serialize_provide() {
        let ipfs_service = "IPFS.get_QmFile";
        let (target, arguments) = BuiltinService::DelegateProviding(DelegateProviding {
            service_id: ipfs_service.into(),
        })
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
            Some(BuiltinService::DelegateProviding(DelegateProviding { service_id })) => {
                assert_eq!(service_id, ipfs_service)
            }
            wrong => unreachable!(
                "target should be Some(BuiltinService::DelegateProviding, was {:?}",
                wrong
            ),
        };
    }

    fn get_cert() -> Certificate {
        Certificate::from_str(
            r#"11
1111
GA9VsZa2Cw2RSZWfxWLDXTLnzVssAYPdrkwT1gHaTJEg
QPdFbzpN94d5tzwV4ATJXXzb31zC7JZ9RbZQ1pbgN3TTDnxuVckCmvbzvPaXYj8Mz7yrL66ATbGGyKqNuDyhXTj
18446744073709551615
1589250571718
D7p1FrGz35dd3jR7PiCT12pAZzxV5PFBcX7GdS9Y8JNb
61QT8JYsAWXjnmUcjJcuNnBHWfrn9pijJ4mX64sDX4N7Knet5jr2FzELrJZAAV1JDZQnATYpGf7DVhnitcUTqpPr
1620808171718
1589250571718
4cEQ2DYGdAbvgrAP96rmxMQvxk9MwtqCAWtzRrwJTmLy
xdHh499gCUD7XA7WLXqCR9ZXxQZFweongvN9pa2egVdC19LJR9814pNReP4MBCCctsGbLmddygT6Pbev1w62vDZ
1620808171719
1589250571719"#,
        )
        .expect("deserialize cert")
    }

    #[test]
    fn serialize_add_certs() {
        let certs = vec![get_cert()];
        let peer_id = RandomPeerId::random();
        let msg_id = uuid::Uuid::new_v4().to_string();
        let (target, arguments) = BuiltinService::AddCertificates(AddCertificates {
            certificates: certs.clone(),
            peer_id: peer_id.clone(),
            msg_id: Some(msg_id.clone()),
        })
        .into();

        let call = gen_provide_call(target, arguments);

        let _service: AddCertificates =
            serde_json::from_value(call.arguments.clone()).expect("deserialize");

        let service =
            BuiltinService::from(&call.target.unwrap(), call.arguments).expect("parse service");

        match service {
            BuiltinService::AddCertificates(add) => {
                assert_eq!(add.certificates, certs);
                assert_eq!(add.peer_id, peer_id);
                assert_eq!(add.msg_id, Some(msg_id));
            }
            _ => unreachable!("expected add certificates"),
        }
    }

    #[test]
    fn serialize_add_certs_only() {
        let certs = vec![get_cert()];
        let peer_id = RandomPeerId::random();
        let msg_id = uuid::Uuid::new_v4().to_string();
        let add = AddCertificates {
            certificates: certs.clone(),
            peer_id: peer_id.clone(),
            msg_id: Some(msg_id.clone()),
        };

        let string = serde_json::to_string(&add).expect("serialize");
        let deserialized = serde_json::from_str(&string).expect("deserialize");

        assert_eq!(add, deserialized);
    }

    #[test]
    fn serialize_get_certs() {
        let peer_id = RandomPeerId::random();
        let msg_id = uuid::Uuid::new_v4().to_string();
        let (target, arguments) = BuiltinService::GetCertificates(GetCertificates {
            peer_id: peer_id.clone(),
            msg_id: Some(msg_id.clone()),
        })
        .into();

        let call = gen_provide_call(target, arguments);

        let _service: GetCertificates =
            serde_json::from_value(call.arguments.clone()).expect("deserialize");

        let service =
            BuiltinService::from(&call.target.unwrap(), call.arguments).expect("parse service");

        match service {
            BuiltinService::GetCertificates(add) => {
                assert_eq!(add.peer_id, peer_id);
                assert_eq!(add.msg_id, Some(msg_id));
            }
            _ => unreachable!("expected get certificates"),
        }
    }
}
