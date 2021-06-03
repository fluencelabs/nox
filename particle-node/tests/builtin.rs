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

#[macro_use]
extern crate fstrings;

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use eyre::WrapErr;
use libp2p::core::Multiaddr;
use libp2p::kad::kbucket::Key;
use libp2p::PeerId;
use maplit::hashmap;
use serde::Deserialize;
use serde_json::{json, Value as JValue};

use fluence_libp2p::RandomPeerId;
use json_utils::into_array;
use particle_protocol::Particle;
use test_utils::{
    create_service, load_module, make_swarms, make_swarms_with_transport_and_mocked_vm, now_ms,
    ConnectedClient, Transport, PARTICLE_TTL,
};

#[derive(Deserialize, Debug)]
struct NodeInfo {
    pub external_addresses: Vec<Multiaddr>,
}

#[test]
fn identify() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle(
        r#"
        (seq
            (call relay ("peer" "identify") [] info)
            (call client ("op" "return") [info])
        ) 
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );

    let info = client.receive_args().wrap_err("receive args").unwrap();
    let info = info.into_iter().next().unwrap();
    let _: NodeInfo = serde_json::from_value(info).unwrap();
}

#[ignore]
#[test]
fn big_identity() {
    let swarms = make_swarms_with_transport_and_mocked_vm(1, Transport::Network);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let mut particle = Particle::default();
    particle.init_peer_id = client.peer_id;
    particle.data = (0..(1024 * 1024 * 20)).map(|_| u8::MAX).collect();
    particle.timestamp = now_ms() as u64;
    particle.ttl = PARTICLE_TTL;
    client.send(particle);

    client.timeout = Duration::from_secs(60);
    client.receive().wrap_err("receive").unwrap();
}

#[test]
fn remove_service() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets"),
    );

    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("srv" "list") [] list_before)
                (call relay ("srv" "remove") [service])
            )
            (seq
                (call relay ("srv" "list") [] list_after)
                (call %init_peer_id% ("op" "return") [list_before list_after])
            )
        )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "service" => json!(tetraplets_service.id),
        },
    );

    use serde_json::Value::Array;

    if let [Array(before), Array(after)] = client.receive_args().unwrap().as_slice() {
        assert_eq!(before.len(), 1);
        assert_eq!(after.len(), 0);
    } else {
        panic!("incorrect args: expected two arrays")
    }
}

#[test]
fn remove_service_by_alias() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets"),
    );

    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("srv" "add_alias") [alias service])
                (seq
                    (call relay ("srv" "list") [] list_before)
                    (call relay ("srv" "remove") [alias])
                )
            )
            (seq
                (call relay ("srv" "list") [] list_after)
                (call %init_peer_id% ("op" "return") [list_before list_after])
            )
        )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "service" => json!(tetraplets_service.id),
            "alias" => json!("some_alias".to_string()),
        },
    );

    use serde_json::Value::Array;

    if let [Array(before), Array(after)] = client.receive_args().unwrap().as_slice() {
        assert_eq!(before.len(), 1);
        assert_eq!(after.len(), 0);
    } else {
        panic!("incorrect args: expected two arrays")
    }
}

#[test]
fn non_owner_remove_service() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let mut client2 = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets"),
    );

    client2.send_particle(
        r#"
        (seq
            (seq
                (call relay ("srv" "list") [] list_before)
                (xor
                    (call relay ("srv" "remove") [service])
                    (call relay ("op" "identity") [%last_error%] error)
                )
            )
            (seq
                (call relay ("srv" "list") [] list_after)
                (call %init_peer_id% ("op" "return") [list_before list_after error.$[0]!])
            )
        )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "service" => json!(tetraplets_service.id),
        },
    );

    use serde_json::Value::{Array, String};

    let args = client2.receive_args().unwrap();
    if let [Array(before), Array(after), String(error)] = args.as_slice() {
        assert_eq!(before.len(), 1);
        assert_eq!(after.len(), 1);
        assert!(error.len() > 0);

        let error: JValue = serde_json::from_str(&error).unwrap();
        let failed_instruction = error.get("instruction").unwrap().as_str().unwrap();
        assert_eq!(
            failed_instruction,
            r#"call relay ("srv" "remove") [service] "#
        );
    } else {
        panic!("incorrect args: expected two arrays, got: {:?}", args)
    }
}

#[test]
fn resolve_alias() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets"),
    );

    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("srv" "add_alias") [alias service])
                (call relay ("srv" "resolve_alias") [alias] result)
            )
            (call %init_peer_id% ("op" "return") [result])
        )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "service" => json!(tetraplets_service.id),
            "alias" => json!("some_alias".to_string()),
        },
    );

    let service_id = client.receive_args().wrap_err("receive args").unwrap();
    let service_id = service_id.into_iter().next().unwrap();
    let service_id: String = serde_json::from_value(service_id).unwrap();

    assert_eq!(tetraplets_service.id, service_id);
}

#[test]
fn resolve_alias_not_exists() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle(
        r#"
        (xor
            (seq
                (call relay ("srv" "resolve_alias") [alias] result)
                (call %init_peer_id% ("op" "return") [result])
            )
            (call %init_peer_id% ("op" "return") [%last_error%])
        )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "alias" => json!("some_alias".to_string()),
        },
    );

    let error = client.receive_args().wrap_err("receive args").unwrap();
    let error = error.into_iter().next().unwrap();
    let error: JValue = serde_json::from_str(error.as_str().unwrap()).unwrap();
    let failed_instruction = error.get("instruction").unwrap().as_str().unwrap();
    assert_eq!(
        failed_instruction,
        r#"call relay ("srv" "resolve_alias") [alias] result"#
    );
}

#[test]
fn resolve_alias_removed() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets"),
    );

    client.send_particle(
        r#"
        (xor
            (seq
                (seq
                    (call relay ("srv" "add_alias") [alias service])
                    (call relay ("srv" "remove") [alias])
                )
                (call relay ("srv" "resolve_alias") [alias] result)
            )
            (call %init_peer_id% ("op" "return") [%last_error%])
        )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "service" => json!(tetraplets_service.id),
            "alias" => json!("some_alias".to_string()),
        },
    );

    let error = client.receive_args().wrap_err("receive args").unwrap();
    let error = error.into_iter().next().unwrap();
    let error: JValue = serde_json::from_str(error.as_str().unwrap()).unwrap();
    let failed_instruction = error.get("instruction").unwrap().as_str().unwrap();
    assert_eq!(
        failed_instruction,
        r#"call relay ("srv" "resolve_alias") [alias] result"#
    );
}

#[test]
fn timestamp_ms() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle(
        r#"
        (seq
            (call relay ("peer" "timestamp_ms") [] result)
            (call client ("op" "return") [result])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );

    let result = client.receive_args().wrap_err("receive args").unwrap();
    let result = result.into_iter().next().unwrap();
    let _: u64 = serde_json::from_value(result).unwrap();
}

#[test]
fn timestamp_sec() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle(
        r#"
        (seq
            (call relay ("peer" "timestamp_sec") [] result)
            (call client ("op" "return") [result])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );

    let result = client.receive_args().wrap_err("receive args").unwrap();
    let result = result.into_iter().next().unwrap();
    let _: u64 = serde_json::from_value(result).unwrap();
}

#[test]
fn base58_string_builtins() {
    let script = r#"
    (seq
        (call relay ("op" "string_to_b58") [string] b58_string_out)
        (seq
            (call relay ("op" "string_from_b58") [b58_string] string_out)
            (call relay ("op" "string_from_b58") [b58_string_out] identity_string)
        )
    )
    "#;

    let string = "hello, this is a string! ДОБРЫЙ ВЕЧЕР КАК СЛЫШНО";
    let b58_string = bs58::encode(string).into_string();
    let args = hashmap! {
        "string" => json!(string),
        "b58_string" => json!(b58_string),
    };

    let result = exec_script(script, args, "b58_string_out string_out identity_string", 1);
    assert_eq!(result[0], JValue::String(b58_string));
    assert_eq!(result[1], JValue::String(string.into()));
    assert_eq!(result[2], JValue::String(string.into()));
}

#[test]
fn base58_bytes_builtins() {
    let script = r#"
    (seq
        (call relay ("op" "bytes_to_b58") [bytes] b58_string_out)
        (seq
            (call relay ("op" "bytes_from_b58") [b58_string] bytes_out)
            (call relay ("op" "bytes_from_b58") [b58_string_out] identity_bytes)
        )
    )
    "#;

    let bytes: Vec<_> = (1..32).map(|i| (200 + i) as u8).collect();
    let b58_string = bs58::encode(&bytes).into_string();
    let args = hashmap! {
        "b58_string" => json!(b58_string),
        "bytes" => json!(bytes),
    };

    let result = exec_script(script, args, "b58_string_out bytes_out identity_bytes", 1);
    // let array = into_array(result).expect("must be an array");
    assert_eq!(result[0], json!(b58_string));
    assert_eq!(result[1], json!(bytes));
    assert_eq!(result[2], json!(bytes));
}

#[test]
fn sha256() {
    use multihash::{Code, MultihashDigest, MultihashGeneric};

    let script = r#"
    (seq
        (seq
            ; hash string to multihash encoded as base58
            (call relay ("op" "sha256_string") [string] string_mhash)
            ; hash string to sha256 digest encoded as base58
            (call relay ("op" "sha256_string") [string true] string_digest)
        )
        (seq
            ; hash string to multihash encoded as byte array
            (call relay ("op" "sha256_string") [string false true] bytes_mhash)
            ; hash string to sha256 digest encoded as byte array
            (call relay ("op" "sha256_string") [string true true] bytes_digest)
        )
    )
    "#;

    let string = "hello, как слышно? ХОРОШО!";
    let sha_256: MultihashGeneric<_> = Code::Sha2_256.digest(string.as_bytes());
    let args = hashmap! {
        "string" => json!(string),
    };

    let result = exec_script(
        script,
        args,
        "string_mhash string_digest bytes_mhash bytes_digest",
        1,
    );

    // multihash as base58
    assert_eq!(
        result[0],
        json!(bs58::encode(sha_256.to_bytes()).into_string())
    );
    // sha256 digest as base58
    assert_eq!(
        result[1],
        json!(bs58::encode(sha_256.digest()).into_string())
    );
    // multihash as byte array
    assert_eq!(result[2], json!(sha_256.to_bytes()));
    // sha256 digest as byte array
    assert_eq!(result[3], json!(sha_256.digest()));
}

#[test]
fn neighborhood() {
    let script = r#"
    (seq
        (seq
            (seq
                (call relay ("op" "string_to_b58") ["key"] key)
                (call relay ("kad" "neighborhood") [key] neighborhood_by_key)
            )
            (seq
                (call relay ("op" "sha256_string") ["key"] mhash)
                (call relay ("kad" "neighborhood") [mhash true] neighborhood_by_mhash)
            )
        )
        (xor
            (call relay ("kad" "neighborhood") [key true])
            (call relay ("op" "identity") [%last_error%] error)
        )
    )
    "#;

    let mut result = exec_script(
        script,
        <_>::default(),
        "neighborhood_by_key neighborhood_by_mhash error",
        2,
    );
    let neighborhood_by_key = into_array(result[0].take())
        .expect("neighborhood is an array")
        .into_iter()
        .map(|v| PeerId::from_str(v.as_str().expect("peerid is string")).expect("peerid is valid"));
    let neighborhood_by_mhash = into_array(result[1].take())
        .expect("neighborhood is an array")
        .into_iter()
        .map(|v| PeerId::from_str(v.as_str().expect("peerid is string")).expect("peerid is valid"));
    let error = into_array(result[2].take())
        .expect("error is wrapped in array")
        .into_iter()
        .next()
        .expect("error is defined");
    let error = error.as_str().expect("error is string");
    assert_eq!(neighborhood_by_key.len(), 1);
    assert_eq!(neighborhood_by_mhash.len(), 1);
    assert!(error.contains("Invalid multihash"));
}

#[test]
fn kad_merge() {
    let target = RandomPeerId::random();
    let left = (1..10).map(|_| RandomPeerId::random()).collect::<Vec<_>>();
    let mut right = (1..10).map(|_| RandomPeerId::random()).collect::<Vec<_>>();
    let count = 10;

    let script = r#"
    (call relay ("kad" "merge") [target left right count] merged)
    "#;

    let args = hashmap! {
        "target" => json!(target.to_base58()),
        "left" => json!(left.iter().map(|id| id.to_base58()).collect::<Vec<_>>()),
        "right" => json!(right.iter().map(|id| id.to_base58()).collect::<Vec<_>>()),
        "count" => json!(count),
    };

    let result = exec_script(script, args, "merged", 1);
    let merged = result.into_iter().next().expect("merged is defined");
    let merged = into_array(merged).expect("merged is an array");
    let merged = merged
        .into_iter()
        .map(|id| {
            PeerId::from_str(id.as_str().expect("peerid is a string")).expect("peerid is correct")
        })
        .collect::<Vec<_>>();

    let target_key = Key::from(target);
    let mut expected = left;
    expected.append(&mut right);
    expected.sort_by_cached_key(|id| target_key.distance(&Key::from(id.clone())));
    expected.truncate(count);

    assert_eq!(expected, merged);
}

#[test]
fn ipfs_multiaddr() {
    let first_maddr = "/ip4/1.1.1.1/tcp/1111";
    let second_maddr = "/ip4/2.2.2.2/tcp/2222";

    let result = exec_script_as_admin(
        r#"
        (seq
            (seq
                (xor
                    (call relay ("ipfs" "get_multiaddr") [])
                    (call relay ("op" "identity") [%last_error%] uninit_error)
                )
                (seq
                    (call relay ("ipfs" "set_multiaddr") [first_maddr])
                    (call relay ("ipfs" "set_multiaddr") [second_maddr] replaced_maddr)
                )
            )
            (seq
                (call relay ("ipfs" "get_multiaddr") [] set_maddr)
                (seq
                    (call relay ("ipfs" "clear_multiaddr") [])
                    (xor
                        (call relay ("ipfs" "get_multiaddr") [])
                        (call relay ("op" "identity") [%last_error%] cleared_error)
                    )
                )
            )
        )
        "#,
        hashmap! {
            "first_maddr" => json!(first_maddr),
            "second_maddr" => json!(second_maddr),
        },
        "uninit_error replaced_maddr set_maddr cleared_error",
        1,
        true,
    );

    let uninit_error = result[0].as_array().unwrap()[0].as_str().unwrap();
    let cleared_error = result[3].as_array().unwrap()[0].as_str().unwrap();

    assert!(uninit_error.contains("ipfs multiaddr isn't set"));
    assert_eq!(result[1], json!(first_maddr));
    assert_eq!(result[2], json!(second_maddr));
    assert!(cleared_error.contains("ipfs multiaddr isn't set"));
}

fn exec_script(
    script: &str,
    args: HashMap<&'static str, JValue>,
    result: &str,
    node_count: usize,
) -> Vec<JValue> {
    exec_script_as_admin(script, args, result, node_count, false)
}

fn exec_script_as_admin(
    script: &str,
    mut args: HashMap<&'static str, JValue>,
    result: &str,
    node_count: usize,
    as_admin: bool,
) -> Vec<JValue> {
    let swarms = make_swarms(node_count);

    let keypair = if as_admin {
        Some(swarms[0].management_keypair.clone())
    } else {
        None
    };
    let mut client = ConnectedClient::connect_with_keypair(swarms[0].multiaddr.clone(), keypair)
        .wrap_err("connect client")
        .unwrap();

    args.insert("relay", json!(client.node.to_string()));

    client.send_particle(
        f!(r#"
        (seq
            {script}
            (call %init_peer_id% ("op" "return") [{result}])
        )
        "#),
        args,
    );

    let result = client.receive_args().wrap_err("receive args").unwrap();

    result
}
