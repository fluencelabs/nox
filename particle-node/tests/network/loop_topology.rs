/*
 * Copyright 2021 Fluence Labs Limited
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

use std::time::Duration;

use eyre::WrapErr;
use maplit::hashmap;
use serde_json::json;
use serde_json::Value as JValue;

use connected_client::ConnectedClient;
use created_swarm::{make_swarms, CreatedSwarm};
use log_utils::enable_logs;

use super::join_stream;

fn permutations(swarms: &[CreatedSwarm]) -> Vec<Vec<(String, u32)>> {
    use itertools::*;

    let pids = swarms.iter().map(|s| s.peer_id.to_string());
    let mut i = 0u32;
    let pids = pids
        .permutations(swarms.len())
        .map(|p| {
            p.into_iter()
                .map(|pid| {
                    i += 1;
                    (pid, i)
                })
                .collect()
        })
        .collect();
    pids
}

pub struct Abuse {
    input: Vec<(String, Vec<Vec<String>>)>,
    output: Vec<JValue>,
}

fn abuse_fold(air: &str) -> Abuse {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let nums: Vec<String> = (1..2).map(|i| i.to_string()).collect();
    let vec = vec![nums.clone(), nums.clone(), nums.clone()];
    let elems: Vec<(String, Vec<Vec<String>>)> = vec![
        ("a".into(), vec.clone()),
        ("a".into(), vec.clone()),
        ("a".into(), vec.clone()),
        ("a".into(), vec.clone()),
        ("a".into(), vec.clone()),
    ];

    println!("elems {}", json!(elems));

    client.send_particle(
        air,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "permutations" => json!(elems),
        },
    );

    client.timeout = Duration::from_secs(1);

    let args = client.receive_args().wrap_err("receive args");
    let args = args.expect(format!("{} failed", json!(elems)).as_str());
    println!("args {}", json!(args));
    let output = match args.into_iter().next() {
        Some(JValue::Array(output)) => output,
        Some(wrong) => panic!("expected output to be array, got {}", json!(wrong)),
        None => panic!("empty result on {}", json!(elems)),
    };

    println!("output {}", json!(output));

    Abuse {
        input: elems,
        output,
    }
}

#[test]
fn fold_fold_fold_par_null() {
    let Abuse { input, output } = abuse_fold(
        r#"
        (new $inner
            (seq
                (seq
                    (fold permutations pair
                        (seq
                            (fold pair.$.[1]! peer_ids
                                (seq
                                    (ap peer_ids $inner)
                                    (next peer_ids)
                                )
                            )
                            (next pair)
                        )
                    )
                    (par
                        (fold $inner ns
                            (next ns)
                        )
                        (null)
                    )
                )
                (seq
                    (call relay ("op" "noop") [])
                    (call client ("return" "") [$inner])
                )
            )
        )
        "#,
    );

    let flat: Vec<Vec<String>> = input
        .into_iter()
        .map(|(_, arr)| arr.into_iter())
        .flatten()
        .collect();

    assert_eq!(json!(flat), json!(output));
}

#[test]
fn fold_fold_fold_par_null_join() {
    let Abuse { input, output } = abuse_fold(
        format!(
            r#"
        (seq
            (seq
                (fold permutations pair
                    (seq
                        (fold pair.$.[1]! peer_ids
                            (seq
                                (ap peer_ids $inner)
                                (next peer_ids)
                            )
                        )
                        (next pair)
                    )
                )
                (par
                    (fold $inner ns
                        (seq
                            (ap ns $result)
                            (next ns)
                        )
                    )
                    (null)
                )
            )
            (seq
                (seq
                    (canon relay $inner #inner)
                    {} ;; join $result stream
                )
                (call client ("return" "") [#joined_result])
            )
        )
        "#,
            join_stream("result", "relay", "#inner.length", "joined_result"),
        )
        .as_str(),
    );

    let flat: Vec<Vec<String>> = input
        .into_iter()
        .map(|(_, arr)| arr.into_iter())
        .flatten()
        .collect();

    assert_eq!(json!(flat), json!(output));
}

#[test]
fn fold_fold_fold_seq_two_par_null_folds() {
    let Abuse { input, output } = abuse_fold(
        format!(
            r#"
        (seq
            (seq
                (fold permutations pair
                    (seq
                        (fold pair.$.[1]! peer_ids
                            (seq
                                (ap peer_ids $inner)
                                (next peer_ids)
                            )
                        )
                        (next pair)
                    )
                )
                (seq
                    (par
                        (fold $inner ns
                            (seq
                                (ap ns $result)
                                (next ns)
                            )
                        )
                        (null)
                    )
                    (par
                        (fold $inner ns
                            (next ns)
                        )
                        (null)
                    )
                )
            )
            (seq
                (seq
                    (canon relay $inner #inner)
                    {} ;; join $result stream
                )
                (call client ("return" "") [#joined_result])
            )
        )
        "#,
            join_stream("result", "relay", "#inner.length", "joined_result")
        )
        .as_str(),
    );

    let flat: Vec<Vec<String>> = input
        .into_iter()
        .map(|(_, arr)| arr.into_iter())
        .flatten()
        .collect();

    assert_eq!(json!(flat), json!(output));
}

#[test]
fn fold_par_same_node_stream() {
    let swarms = make_swarms(3);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    for (i, swarm) in swarms.iter().enumerate() {
        if i == 0 {
            log::info!("swarm[{}] = {} (relay)", i, swarm.peer_id)
        } else {
            log::info!("swarm[{}] = {}", i, swarm.peer_id)
        }
    }

    let pid_permutations = permutations(&swarms);
    let mut permutations = pid_permutations.clone().into_iter();
    let permutations = &mut permutations;
    let per_node = permutations.len() / swarms.len();
    let permutations = swarms.iter().fold(vec![], |mut acc, swarm| {
        let perms = permutations.take(per_node).collect::<Vec<_>>();
        assert_eq!(perms.len(), per_node);
        acc.push((swarm.peer_id.to_string(), perms));
        acc
    });

    let flat: Vec<_> = pid_permutations.into_iter().flatten().collect();

    client.timeout = Duration::from_secs(200);
    client.particle_ttl = Duration::from_secs(400);

    client.send_particle(
        format!(
            r#"
        (seq
            (seq
                (null)
                (seq
                    (fold permutations pair
                        (seq
                            (fold pair.$.[1]! peer_ids
                                (seq
                                    (seq
                                        (call pair.$.[0]! ("op" "noop") [])
                                        (ap peer_ids $inner)
                                    )
                                    (next peer_ids)
                                )
                            )
                            (next pair)
                        )
                    )
                    (seq
                        (canon relay $inner #inner)
                        (fold $inner ns
                            (par
                                (fold ns pair
                                    (seq
                                        (seq
                                            (call pair.$.[0]! ("op" "noop") [])
                                            (ap pair.$.[1]! $result)
                                        )
                                        (next pair)
                                    )
                                )
                                (next ns)
                            )
                        )
                    )
                )
            )
            (seq                
                {}
                (call client ("return" "") [#inner #joined_result])
            )
        )
        "#,
            join_stream("result", "relay", "flat_length", "joined_result")
        )
        .as_str(),
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "permutations" => json!(permutations),
            "flat_length" => dbg!(json!(flat.len()))
        },
    );

    let mut args = client.receive_args().wrap_err("receive args").unwrap();
    type Inner = Vec<Vec<(String, u32)>>;
    type Res = Vec<u32>;

    let inner: Inner = serde_json::from_value(args.remove(0)).unwrap();
    let permutations: Inner = permutations
        .into_iter()
        .map(|(_, perms)| perms.into_iter())
        .flatten()
        .collect();
    assert_eq!(permutations, inner);

    let mut res: Res = serde_json::from_value(args.remove(0)).unwrap();
    assert_eq!(flat.len(), res.len());
    let flat: Res = flat.into_iter().map(|(_, i)| i).collect();
    res.sort();
    assert_eq!(flat, res);
}

#[test]
fn fold_seq_join() {
    let swarm = make_swarms(1).remove(0);

    let mut client = ConnectedClient::connect_to(swarm.multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let array: Vec<_> = (1..10).collect();

    client.send_particle(
        r#"
    (seq
        (seq
            (fold array e
                (seq
                    (ap e $stream)
                    (next e)
                )    
            )
            (fold $stream e
                (seq
                    (canon relay $stream #can)
                    (xor
                        (match #can.length array_length
                            (null)
                        )
                        (next e)
                    )
                )
            )
        )
        (seq
            (canon relay $stream #can)
            (call %init_peer_id% ("op" "return") [#can])
        )
    )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "array" => json!(array),
            "array_length" => json!(array.len())
        },
    );

    let arg = client.receive_args().expect("receive args").remove(0);
    let can: Vec<u32> = serde_json::from_value(arg).unwrap();
    assert_eq!(can, array);
}

#[test]
fn fold_seq_same_node_stream() {
    let swarms = make_swarms(3);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    for (i, swarm) in swarms.iter().enumerate() {
        if i == 0 {
            log::info!("swarm[{}] = {} (relay)", i, swarm.peer_id)
        } else {
            log::info!("swarm[{}] = {}", i, swarm.peer_id)
        }
    }

    let pid_permutations = permutations(&swarms);
    let mut permutations = pid_permutations.clone().into_iter();
    let permutations = &mut permutations;
    let per_node = permutations.len() / swarms.len();
    let permutations = swarms.iter().fold(vec![], |mut acc, swarm| {
        let perms = permutations.take(per_node).collect::<Vec<_>>();
        assert_eq!(perms.len(), per_node);
        acc.push((swarm.peer_id.to_string(), perms));
        acc
    });

    let flat: Vec<_> = pid_permutations.into_iter().flatten().collect();

    client.timeout = Duration::from_secs(200);
    client.particle_ttl = Duration::from_secs(400);

    client.send_particle(
        format!(
            r#"
        (seq
            (seq
                (null)
                (seq
                    (fold permutations pair
                        (seq
                            (fold pair.$.[1]! peer_ids
                                (seq
                                    (seq
                                        (call pair.$.[0]! ("op" "noop") [])
                                        (ap peer_ids $inner)
                                    )
                                    (next peer_ids)
                                )
                            )
                            (next pair)
                        )
                    )
                    (seq
                        (canon relay $inner #inner)
                        (fold $inner ns
                            (seq
                                (fold ns pair
                                    (seq
                                        (seq
                                            (call pair.$.[0]! ("op" "noop") [])
                                            (ap pair.$.[1]! $result)
                                        )
                                        (seq
                                            (canon relay $result #mon_res)
                                            (xor
                                                (match #mon_res.length flat_length
                                                    (null)
                                                )
                                                (next pair)
                                            )
                                        )
                                    )
                                )
                                (seq
                                    (canon relay $result #mon_res)
                                    (xor
                                        (match #mon_res.length flat_length
                                            (null)
                                        )
                                        (next ns)
                                    )
                                )
                            )
                        )
                    )
                )
            )
            (seq                
                (call relay ("op" "noop") [])
                (seq
                    (canon client $result #end_result)
                    (call client ("return" "") [#inner #end_result])
                )
            )
        )
        "#
        )
        .as_str(),
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "permutations" => json!(permutations),
            "flat_length" => dbg!(json!(flat.len()))
        },
    );

    let mut args = client.receive_args().wrap_err("receive args").unwrap();
    type Inner = Vec<Vec<(String, u32)>>;
    type Res = Vec<u32>;

    let inner: Inner = serde_json::from_value(args.remove(0)).unwrap();
    let permutations: Inner = permutations
        .into_iter()
        .map(|(_, perms)| perms.into_iter())
        .flatten()
        .collect();
    assert_eq!(permutations, inner);

    let res: Res = serde_json::from_value(args.remove(0)).unwrap();
    assert_eq!(flat.len(), res.len());
    let flat: Res = flat.into_iter().map(|(_, i)| i).collect();
    assert_eq!(flat, res);
}

#[test]
fn par_wait_two() {
    let swarms = make_swarms(4);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle_ext(
        r#"
        (xor
         (seq
          (seq
           (seq
            (seq
             (seq
              (call %init_peer_id% ("getDataSrv" "-relay-") [] -relay-)
              (call %init_peer_id% ("getDataSrv" "relay") [] relay)
             )
             (call -relay- ("op" "noop") [])
            )
            (xor
             (seq
              (seq
               (seq
                (seq
                 (seq
                  (call relay ("op" "string_to_b58") [%init_peer_id%] k)
                  (call relay ("kad" "neighborhood") [k [] []] nodes)
                 )
                 (fold nodes n
                  (par
                   (seq
                    (xor
                     (call n ("peer" "timestamp_sec") [] $res)
                     (null)
                    )
                    (call relay ("op" "noop") [])
                   )
                   (next n)
                  )
                 )
                )
                (call relay ("op" "identity") [$res.$.[0]!])
               )
               (call relay ("op" "identity") [$res.$.[1]!])
              )
              (call relay ("op" "identity") [$res.$.[2]!])
             )
             (seq
              (call -relay- ("op" "noop") [])
              (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 1])
             )
            )
           )
           (call -relay- ("op" "noop") [])
          )
          (xor
           (call %init_peer_id% ("callbackSrv" "response") [$res])
           (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 2])
          )
         )
         (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 3])
        )
        "#,
        hashmap! {
            "-relay-" => json!(client.node.to_base58()),
            "relay" => json!(client.node.to_base58()),
        },
        true,
    );

    client.receive().unwrap();
}

#[test]
fn fold_via() {
    let swarms = make_swarms(4);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle_ext(
        r#"
        (xor
         (seq
          (seq
           (seq
            (seq
             (seq
              (seq
               (seq
                (seq
                 (call %init_peer_id% ("getDataSrv" "-relay-") [] -relay-)
                 (call %init_peer_id% ("getDataSrv" "node_id") [] node_id)
                )
                (call %init_peer_id% ("getDataSrv" "viaAr") [] viaAr)
               )
               (call -relay- ("op" "noop") [])
              )
              (fold viaAr -via-peer-
               (seq
                (call -via-peer- ("op" "noop") [])
                (next -via-peer-)
               )
              )
             )
             (xor
              (call node_id ("peer" "identify") [] p)
              (seq
               (seq
                (seq
                 (fold viaAr -via-peer-
                  (seq
                   (call -via-peer- ("op" "noop") [])
                   (next -via-peer-)
                  )
                 )
                 (call -relay- ("op" "noop") [])
                )
                (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 1])
               )
               (call -relay- ("op" "noop") [])
              )
             )
            )
            (fold viaAr -via-peer-
             (seq
              (call -via-peer- ("op" "noop") [])
              (next -via-peer-)
             )
            )
           )
           (call -relay- ("op" "noop") [])
          )
          (xor
           (call %init_peer_id% ("callbackSrv" "response") [p])
           (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 2])
          )
         )
         (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 3])
        )
        "#,
        hashmap! {
            "-relay-" => json!(client.node.to_base58()),
            "node_id" => json!(client.node.to_base58()),
            "viaAr" => json!(swarms.iter().map(|s| s.peer_id.to_string()).collect::<Vec<_>>()),
        },
        true,
    );

    client.receive().unwrap();
}

#[test]
fn join_empty_stream() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle(
        r#"
        (seq
            (xor
                (call relay ("op" "noop") [])
                (call %init_peer_id% ("op" "identity") [""] $ns)
            )
            (call %init_peer_id% ("op" "return") [$ns.$.[0]! $ns.$.[1]! $ns])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "nodes" => json!(swarms.iter().map(|s| s.peer_id.to_base58()).collect::<Vec<_>>()),
        },
    );

    let err = client.receive_args().err().expect("receive error");
    assert_eq!(
        err.to_string(),
        "Received a particle, but it didn't return anything"
    );
}
