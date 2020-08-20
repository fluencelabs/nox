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

use crate::app_service::Blueprint;
use faas_api::Address;
use fluence_app_service::RawModuleConfig;
use fluence_libp2p::peerid_serializer;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use trust_graph::{certificate_serde, Certificate};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provide {
    pub name: String,
    pub address: Address,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct Identify {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct GetInterface {
    pub service_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct GetActiveInterfaces {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct GetAvailableModules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddModule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
    #[serde(
        deserialize_with = "base64::de_bytes",
        serialize_with = "base64::ser_bytes"
    )]
    pub bytes: Vec<u8>,
    pub config: RawModuleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBlueprint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
    pub blueprint: Blueprint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateService {
    pub blueprint_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct GetAvailableBlueprints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
}

mod base64 {
    use serde::{Deserialize, Serialize, Serializer};

    pub fn de_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        base64::decode(&s)
            .map_err(|e| serde::de::Error::custom(format!("base64 decode error: {:?}", e)))
    }

    pub fn ser_bytes<S>(bs: &[u8], ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        String::serialize(&base64::encode(bs), ser)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BuiltinService {
    Provide(Provide),
    GetCertificates(GetCertificates),
    AddCertificates(AddCertificates),
    Identify(Identify),
    GetInterface(GetInterface),
    GetActiveInterfaces(GetActiveInterfaces),
    GetAvailableModules(GetAvailableModules),
    AddModule(AddModule),
    AddBlueprint(AddBlueprint),
    CreateService(CreateService),
    GetAvailableBlueprints(GetAvailableBlueprints),
}

#[derive(Debug)]
pub enum BuiltinServiceError {
    Serde(serde_json::Error),
    UnknownService(String),
}

impl<'a> From<serde_json::Error> for BuiltinServiceError {
    fn from(err: serde_json::Error) -> Self {
        BuiltinServiceError::Serde(err)
    }
}

impl<'a> Display for BuiltinServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuiltinServiceError::Serde(serderr) => serderr.fmt(f),
            BuiltinServiceError::UnknownService(s) => write!(f, "unknown builtin service `{}`", s),
        }
    }
}

impl BuiltinService {
    const PROVIDE: &'static str = "provide";
    const CERTS: &'static str = "certificates";
    const ADD_CERTS: &'static str = "add_certificates";
    const IDENTIFY: &'static str = "identify";
    const GET_INTERFACE: &'static str = "get_interface";
    const GET_ACTIVE_INTERFACES: &'static str = "get_active_interfaces";
    const GET_AVAILABLE_MODULES: &'static str = "get_available_modules";
    const ADD_MODULE: &'static str = "add_module";
    const ADD_BLUEPRINT: &'static str = "add_blueprint";
    const CREATE_SERVICE: &'static str = "create";
    const GET_AVAILABLE_BLUEPRINTS: &'static str = "get_available_blueprints";
    #[rustfmt::skip]
    const SERVICES: &'static [&'static str] = &[
        Self::PROVIDE, Self::CERTS, Self::ADD_CERTS, Self::IDENTIFY, Self::GET_INTERFACE,
        Self::GET_ACTIVE_INTERFACES, Self::GET_AVAILABLE_MODULES, Self::ADD_MODULE,
        Self::ADD_BLUEPRINT, Self::CREATE_SERVICE, Self::GET_AVAILABLE_BLUEPRINTS,
    ];

    #[allow(clippy::needless_lifetimes)]
    pub fn from(
        service_id: String,
        arguments: serde_json::Value,
    ) -> Result<Self, BuiltinServiceError> {
        use serde_json::from_value;
        use BuiltinService::*;

        let service = match service_id.as_str() {
            Self::PROVIDE => Provide(from_value(arguments)?),
            Self::CERTS => GetCertificates(from_value(arguments)?),
            Self::ADD_CERTS => AddCertificates(from_value(arguments)?),
            Self::IDENTIFY => Identify(from_value(arguments)?),
            Self::GET_INTERFACE => GetInterface(from_value(arguments)?),
            Self::GET_ACTIVE_INTERFACES => GetActiveInterfaces(from_value(arguments)?),
            Self::GET_AVAILABLE_MODULES => GetAvailableModules(from_value(arguments)?),
            Self::ADD_MODULE => AddModule(from_value(arguments)?),
            Self::ADD_BLUEPRINT => AddBlueprint(from_value(arguments)?),
            Self::CREATE_SERVICE => CreateService(from_value(arguments)?),
            Self::GET_AVAILABLE_BLUEPRINTS => GetAvailableBlueprints(from_value(arguments)?),
            _ => return Err(BuiltinServiceError::UnknownService(service_id)),
        };

        Ok(service)
    }

    pub fn is_builtin(service: &str) -> bool {
        Self::SERVICES.contains(&service)
    }

    #[cfg(test)]
    pub fn as_target_args(&self) -> (&str, serde_json::Value) {
        use serde_json::json;

        let service_id = match self {
            BuiltinService::Provide { .. } => BuiltinService::PROVIDE,
            BuiltinService::GetCertificates { .. } => BuiltinService::CERTS,
            BuiltinService::AddCertificates { .. } => BuiltinService::ADD_CERTS,
            BuiltinService::Identify { .. } => BuiltinService::IDENTIFY,
            BuiltinService::GetInterface { .. } => BuiltinService::GET_INTERFACE,
            BuiltinService::GetActiveInterfaces { .. } => BuiltinService::GET_ACTIVE_INTERFACES,
            BuiltinService::GetAvailableModules { .. } => BuiltinService::GET_AVAILABLE_MODULES,
            BuiltinService::AddModule(_) => BuiltinService::ADD_MODULE,
            BuiltinService::AddBlueprint(_) => BuiltinService::ADD_BLUEPRINT,
            BuiltinService::CreateService(_) => BuiltinService::CREATE_SERVICE,
            BuiltinService::GetAvailableBlueprints(_) => BuiltinService::GET_AVAILABLE_BLUEPRINTS,
        };

        (service_id, json!(self))
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use faas_api::FunctionCall;
    use fluence_libp2p::RandomPeerId;
    use std::str::FromStr;

    #[test]
    fn serialize_provide() {
        let ipfs_service = "IPFS.get_QmFile";
        let addr = Address::random_relay();
        let service = BuiltinService::Provide(Provide {
            name: ipfs_service.into(),
            address: addr.clone(),
        });
        let (service_id, arguments) = service.as_target_args();
        let mut call = FunctionCall::random();
        call.module = service_id.to_string().into();
        call.arguments = arguments;

        assert_eq!(service_id, "provide");

        match BuiltinService::from(service_id.into(), call.arguments) {
            Ok(BuiltinService::Provide(Provide { name, address })) => {
                assert_eq!(name, ipfs_service);
                assert_eq!(addr, address);
            }
            wrong => unreachable!(
                "target should be Some(BuiltinService::Provide, was {:?}",
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
        let service = BuiltinService::AddCertificates(AddCertificates {
            certificates: certs.clone(),
            peer_id: peer_id.clone(),
            msg_id: Some(msg_id.clone()),
        });
        let (service_id, arguments) = service.as_target_args();

        let mut call = FunctionCall::random();
        call.target = Some(Address::random());
        call.arguments = arguments;
        call.module = Some(service_id.to_string());

        let _service: AddCertificates =
            serde_json::from_value(call.arguments.clone()).expect("deserialize");

        let service = BuiltinService::from(call.module.unwrap(), call.arguments).unwrap();

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
        let service = BuiltinService::GetCertificates(GetCertificates {
            peer_id: peer_id.clone(),
            msg_id: Some(msg_id.clone()),
        });
        let (service_id, arguments) = service.as_target_args();

        let mut call = FunctionCall::random();
        call.target = Address::random().into();
        call.arguments = arguments;
        call.module = service_id.to_string().into();

        let _service: GetCertificates =
            serde_json::from_value(call.arguments.clone()).expect("deserialize");

        let service = BuiltinService::from(call.module.unwrap(), call.arguments).unwrap();

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
        let service = BuiltinService::Identify(Identify {
            msg_id: msg_id.clone(),
        });
        let (service_id, arguments) = service.as_target_args();
        let mut call = FunctionCall::random();
        call.target = Address::random().into();
        call.arguments = arguments;
        call.module = service_id.to_string().into();
        let service = BuiltinService::from(service_id.into(), call.arguments).unwrap();

        match service {
            BuiltinService::Identify(identify) => assert_eq!(msg_id, identify.msg_id),
            _ => unreachable!("expected identify"),
        }
    }
}
