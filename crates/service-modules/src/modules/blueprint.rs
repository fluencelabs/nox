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

use cid_utils::Hash;
use libipld::codec::Codec;
use libipld::json::DagJsonCodec;
use libipld::Ipld;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AddBlueprint {
    pub name: String,
    pub dependencies: Vec<Hash>,
}

impl AddBlueprint {
    pub fn new(name: String, dependencies: Vec<Hash>) -> Self {
        Self { name, dependencies }
    }

    pub fn get_ipld(&self) -> Ipld {
        let mut map = BTreeMap::new();
        map.insert("name".to_string(), Ipld::String(self.name.clone()));
        map.insert(
            "dependencies".to_string(),
            Ipld::List(
                self.dependencies
                    .clone()
                    .into_iter()
                    .map(|h| Ipld::Link(h.0))
                    .collect(),
            ),
        );

        Ipld::Map(map)
    }

    /// encode IPLD object with DAG JSON codec
    pub fn encode(&self) -> eyre::Result<Vec<u8>> {
        DagJsonCodec
            .encode(&self.get_ipld())
            .map_err(|e| eyre::eyre!(e))
    }

    pub fn to_string(&self) -> eyre::Result<String> {
        Ok(String::from_utf8(self.encode()?)?)
    }

    pub fn decode(data: &[u8]) -> eyre::Result<Self> {
        let ipld: Ipld = DagJsonCodec.decode(data).map_err(|e| eyre::eyre!(e))?;
        let name = ipld
            .get("name")
            .map_err(|_| eyre::eyre!("name field is missing"))?;
        let dependencies = ipld
            .get("dependencies")
            .map_err(|_| eyre::eyre!("dependencies field is missing"))?;

        let name = match name {
            Ipld::String(s) => s.clone(),
            _ => return Err(eyre::eyre!("name field is not a string")),
        };

        let dependencies = match dependencies {
            Ipld::List(l) => l
                .iter()
                .map(|ipld| match ipld {
                    Ipld::Link(h) => Ok(Hash(*h)),
                    _ => Err(eyre::eyre!("dependency is not a link")),
                })
                .collect::<eyre::Result<Vec<_>>>()?,
            _ => return Err(eyre::eyre!("dependencies field is not a list")),
        };

        Ok(Self { name, dependencies })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blueprint {
    pub name: String,
    pub id: String,
    pub dependencies: Vec<Hash>,
}

impl Blueprint {
    pub fn new(add_blueprint: AddBlueprint) -> eyre::Result<Self> {
        let id = Hash::new_bytes(&add_blueprint.encode()?)?.to_string();

        Ok(Self {
            name: add_blueprint.name,
            id,
            dependencies: add_blueprint.dependencies,
        })
    }

    pub fn get_facade_module(&self) -> Option<Hash> {
        self.dependencies.last().cloned()
    }
}

#[test]
fn test_blueprint_hash() {
    let cid1 =
        Hash::from_string("bafybeiey4i2vtj7uu7tlvdoc2o52uuuwxa4ahcx5g4lpqzk4qtd5klniuq").unwrap();
    let cid2 =
        Hash::from_string("bafybeibuvzascfzi5ikyzhjxdkridgytg4z26ujtnx7xrejq7gxq54ssdm").unwrap();
    let blueprint = Blueprint::new(AddBlueprint {
        name: "trust-graph".to_string(),
        dependencies: vec![cid1, cid2],
    })
    .unwrap();
    assert_eq!(
        blueprint.id.to_string(),
        "bafyreicpdnxse3ubhxeuxuzoanj22xs6m2jfmx6475zga5ntjtk4d6ygpi"
    );
}
