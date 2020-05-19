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

use faas_api::Protocol::{self, *};
use fluence_libp2p::peerid_serializer;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identify {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BuiltinService {
    DelegateProviding(DelegateProviding),
    GetCertificates(GetCertificates),
    AddCertificates(AddCertificates),
    Identify(Identify),
}

#[derive(Debug)]
pub enum Error<'a> {
    Serde(serde_json::Error),
    UnknownService(&'a str),
    InvalidProtocol(&'a Protocol),
}

impl<'a> From<serde_json::Error> for Error<'a> {
    fn from(err: serde_json::Error) -> Self {
        Error::Serde(err)
    }
}

impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Serde(serderr) => serderr.fmt(f),
            Error::UnknownService(s) => write!(f, "unknown builtin service `{}`", s),
            Error::InvalidProtocol(p) => {
                write!(f, "invalid protocol: expected /service, got `{}`", p)
            }
        }
    }
}

impl BuiltinService {
    const PROVIDE: &'static str = "provide";
    const CERTS: &'static str = "certificates";
    const ADD_CERTS: &'static str = "add_certificates";
    const IDENTIFY: &'static str = "identify";
    const SERVICES: [&'static str; 4] =
        [Self::PROVIDE, Self::CERTS, Self::ADD_CERTS, Self::IDENTIFY];

    pub fn from<'a>(target: &'a Protocol, arguments: serde_json::Value) -> Result<Self, Error<'a>> {
        // Check it's `/service/ID` and `ID` is one of builtin services
        let service_id = match target {
            Service(service_id) => service_id,
            _ => return Err(Error::InvalidProtocol(target)),
        };

        let service = match service_id.as_str() {
            Self::PROVIDE => BuiltinService::DelegateProviding(serde_json::from_value(arguments)?),
            Self::CERTS => BuiltinService::GetCertificates(serde_json::from_value(arguments)?),
            Self::ADD_CERTS => BuiltinService::AddCertificates(serde_json::from_value(arguments)?),
            Self::IDENTIFY => BuiltinService::Identify(serde_json::from_value(arguments)?),
            s => return Err(Error::UnknownService(s)),
        };

        Ok(service)
    }

    pub fn is_builtin(service: &Protocol) -> bool {
        match service {
            Service(service_id) => Self::SERVICES.contains(&service_id.as_str()),
            _ => false,
        }
    }
}

impl Into<(Protocol, serde_json::Value)> for BuiltinService {
    fn into(self) -> (Protocol, serde_json::Value) {
        use serde_json::json;

        let service_id = match self {
            BuiltinService::DelegateProviding { .. } => BuiltinService::PROVIDE,
            BuiltinService::GetCertificates { .. } => BuiltinService::CERTS,
            BuiltinService::AddCertificates { .. } => BuiltinService::ADD_CERTS,
            BuiltinService::Identify { .. } => BuiltinService::IDENTIFY,
        };
        let target = Protocol::Service(service_id.to_string());
        let arguments = json!(self);

        (target, arguments)
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
        let target = &target;
        let call = gen_provide_call(target.into(), arguments);

        let protocols = call.target.as_ref().expect("non empty").protocols();

        let service_id = match protocols.as_slice() {
            [Protocol::Service(service_id)] => service_id,
            wrong => unreachable!("target should be Address::Service, was {:?}", wrong),
        };

        assert_eq!(service_id, "provide");

        match BuiltinService::from(target, call.arguments) {
            Ok(BuiltinService::DelegateProviding(DelegateProviding { service_id })) => {
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

        let call = gen_provide_call(target.into(), arguments);

        let _service: AddCertificates =
            serde_json::from_value(call.arguments.clone()).expect("deserialize");

        let target = call.target.unwrap().pop_front().unwrap();
        let service = BuiltinService::from(&target, call.arguments).unwrap();

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
            certificates: certs,
            peer_id,
            msg_id: Some(msg_id),
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

        let call = gen_provide_call(target.into(), arguments);

        let _service: GetCertificates =
            serde_json::from_value(call.arguments.clone()).expect("deserialize");

        let target = call.target.unwrap().pop_front().unwrap();
        let service = BuiltinService::from(&target, call.arguments).unwrap();

        match service {
            BuiltinService::GetCertificates(add) => {
                assert_eq!(add.peer_id, peer_id);
                assert_eq!(add.msg_id, Some(msg_id));
            }
            _ => unreachable!("expected get certificates"),
        }
    }

    #[test]
    fn serialize_identify() {
        let msg_id = Some(uuid::Uuid::new_v4().to_string());
        let (target, arguments) = BuiltinService::Identify(Identify {
            msg_id: msg_id.clone(),
        })
        .into();
        let call = gen_provide_call(target.into(), arguments);
        let target = call.target.unwrap().pop_front().unwrap();
        let service = BuiltinService::from(&target, call.arguments).unwrap();

        match service {
            BuiltinService::Identify(identify) => assert_eq!(msg_id, identify.msg_id),
            _ => unreachable!("expected identify"),
        }
    }
}
