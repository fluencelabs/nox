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

use connected_client::ConnectedClient;
use created_swarm::{
    make_swarms, make_swarms_with_keypair, make_swarms_with_transport_and_mocked_vm,
};
use fluence_libp2p::RandomPeerId;
use fluence_libp2p::Transport;
use json_utils::into_array;
use now_millis::now_ms;
use particle_protocol::Particle;
use service_modules::load_module;
use test_constants::PARTICLE_TTL;

use libp2p::core::Multiaddr;
use libp2p::kad::kbucket::Key;

use eyre::WrapErr;
use fluence_keypair::KeyPair;
use itertools::Itertools;
use libp2p::PeerId;
use maplit::hashmap;
use serde::Deserialize;
use serde_json::{json, Value as JValue};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use test_utils::create_service;

#[derive(Deserialize, Debug)]
struct NodeInfo {
    #[allow(dead_code)]
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
    let _: NodeInfo =
        serde_json::from_value(info[0].clone()).expect(&format!("deserialize {:?}", info[0]));
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
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
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
fn remove_service_restart() {
    let kp = KeyPair::generate_ed25519();
    let swarms = make_swarms_with_keypair(1, kp.clone());

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
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

    // stop swarm
    swarms.into_iter().map(|s| s.outlet.send(())).for_each(drop);
    let swarms = make_swarms_with_keypair(1, kp.clone());
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle(
        r#"
        (seq
            (call relay ("srv" "list") [] list_after)
            (call %init_peer_id% ("op" "return") [list_after])
        )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "service" => json!(tetraplets_service.id),
        },
    );

    if let [Array(after)] = client.receive_args().unwrap().as_slice() {
        assert_eq!(after.len(), 0);
    } else {
        panic!("incorrect args: expected array")
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
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
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
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    );

    client2.send_particle(
        r#"
        (seq
            (seq
                (call relay ("srv" "list") [] list_before)
                (xor
                    (call relay ("srv" "remove") [service])
                    (ap %last_error%.$.instruction error)
                )
            )
            (seq
                (call relay ("srv" "list") [] list_after)
                (call %init_peer_id% ("op" "return") [list_before list_after error])
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

        assert_eq!(error, r#"call relay ("srv" "remove") [service] "#);
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
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
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
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "alias" => json!("some_alias".to_string()),
        },
    );

    let error = client.receive_args().wrap_err("receive args").unwrap();
    let error = error.into_iter().next().unwrap();
    let failed_instruction = error.as_str().unwrap();
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
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
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
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
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
    let failed_instruction = error.as_str().unwrap();
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
            (ap %last_error%.$.message error)
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
    let error = result[2].take();
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
    right = right.into_iter().chain(left.clone().into_iter()).collect();
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
    expected = expected.into_iter().unique().collect();
    expected.sort_by_cached_key(|id| target_key.distance(&Key::from(id.clone())));
    expected.truncate(count);

    assert_eq!(expected, merged);
}

#[test]
fn noop() {
    let result = exec_script(
        r#"(call relay ("op" "noop") ["hi"] result)"#,
        <_>::default(),
        "result",
        1,
    );
    assert_eq!(result, vec![json!("")])
}

#[test]
fn identity() {
    let result = exec_script(
        r#"(call relay ("op" "identity") ["hi"] result)"#,
        <_>::default(),
        "result",
        1,
    );
    assert_eq!(result, vec![json!("hi")]);

    let error = exec_script(
        r#"
        (xor
            (call relay ("op" "identity") ["hi" "there"] result)
            (ap %last_error%.$.message error)
        )
        "#,
        <_>::default(),
        "error",
        1,
    );
    let error = error[0].as_str().unwrap();
    assert!(error.contains("identity accepts up to 1 arguments, received 2 arguments"));
}

#[test]
fn array() {
    let result = exec_script(
        r#"(call relay ("op" "array") ["hi"] result)"#,
        <_>::default(),
        "result",
        1,
    );
    assert_eq!(result, vec![json!(["hi"])])
}

#[test]
fn concat() {
    let result = exec_script(
        r#"(call relay ("op" "concat") [zerozero one empty two three fourfive empty] result)"#,
        hashmap! {
            "zerozero" => json!([0, 0]),
            "empty" => json!([]),
            "one" => json!([1]),
            "two" => json!([2]),
            "three" => json!([3]),
            "fourfive" => json!([4,5]),
        },
        "result",
        1,
    );
    assert_eq!(result, vec![json!([0, 0, 1, 2, 3, 4, 5])])
}

#[test]
fn array_length() {
    let result = exec_script(
        r#"
        (seq
            (seq
                (call relay ("op" "array_length") [empty_array] zero)
                (call relay ("op" "array_length") [five_array] five)
            )
            (seq
                (xor
                    (call relay ("op" "array_length") [])
                    (ap %last_error%.$.message zero_error)
                )
                (seq
                    (xor
                        (call relay ("op" "array_length") [empty_array five_array])
                        (ap %last_error%.$.message count_error)
                    )
                    (xor
                        (call relay ("op" "array_length") ["hola"])
                        (ap %last_error%.$.message type_error)
                    )
                )
            )
        )
        "#,
        hashmap! {
            "empty_array" => json!([]),
            "five_array" => json!([1, 2, 3, 4, 5])
        },
        "zero five zero_error count_error type_error",
        1,
    );

    assert_eq!(result, vec![
        json!(0),
        json!(5),
        json!("Local service error, ret_code is 1, error message is '\"op array_length accepts exactly 1 argument: 0 found\"'"),
        json!("Local service error, ret_code is 1, error message is '\"op array_length accepts exactly 1 argument: 2 found\"'"),
        json!("Local service error, ret_code is 1, error message is '\"op array_length's argument must be an array\"'"),
    ])
}

#[test]
fn timeout_race() {
    let fast_result = exec_script(
        r#"
        (par
            (call relay ("peer" "timeout") [1000 "slow_result"] $result)
            ;;(ap "fast_result" $result)
            (call relay ("op" "identity") ["fast_result"] $result)
            ;;(call relay ("peer" "timeout") [2000 "very_slow_result"] $result)
        )
    "#,
        <_>::default(),
        "$result.$[0]",
        1,
    );

    assert_eq!(&fast_result[0], "fast_result");
}

#[test]
fn timeout_wait() {
    let slow_result = exec_script(
        r#"
        (seq
            (par
                (call relay ("peer" "timeout") [1000 "timed_out"] $ok_or_err)
                (call "invalid_peer" ("op" "identity") ["never"] $ok_or_err) 
            )
            (xor
                (match $ok_or_err.$[0] "timed_out"
                    (ap "timed out" $result)
                )
                (ap "impossible happened" $result)
            )
        )
    "#,
        <_>::default(),
        "$result.$[0]",
        1,
    );

    assert_eq!(&slow_result[0], "timed out");
}

#[test]
fn debug_stringify() {
    fn stringify(value: impl Into<JValue>) -> String {
        let mut result = exec_script(
            r#"(call relay ("debug" "stringify") [value] result)"#,
            hashmap! {
                "value" => value.into()
            },
            "result",
            1,
        );

        result[0].take().as_str().unwrap().to_string()
    }

    assert_eq!(stringify("hello"), r#""hello""#);
    assert_eq!(stringify(101), r#"101"#);
    assert_eq!(stringify(json!({ "a": "b" })), r#"{"a":"b"}"#);
    assert_eq!(stringify(json!(["a"])), r#"["a"]"#);
    assert_eq!(stringify(json!(["a", "b"])), r#"["a","b"]"#);

    let result = exec_script(
        r#"(call relay ("debug" "stringify") [] result)"#,
        <_>::default(),
        "result",
        1,
    );
    assert_eq!(
        result[0].as_str().unwrap().to_string(),
        r#""<empty argument list>""#
    );

    let result = exec_script(
        r#"(call relay ("debug" "stringify") ["a" "b"] result)"#,
        <_>::default(),
        "result",
        1,
    );
    assert_eq!(result[0].as_str().unwrap().to_string(), r#"["a","b"]"#);
}

#[test]
fn xor_type_error() {
    let result = exec_script(
        r#"
        (xor
            (call relay ("dist" "make_module_config") [obj obj obj])
            (call relay ("op" "identity") [%last_error%] error)
        )
        "#,
        hashmap! {
            "obj" => json!({"never valid": "ever"}),
        },
        "error",
        1,
    );
    assert_eq!(
        result[0].get("error_code"),
        Some(JValue::Number(10000.into())).as_ref()
    )
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
