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

use connected_client::{ClientEvent, ConnectedClient};
use created_swarm::{make_swarms, CreatedSwarm};
use test_constants::KAD_TIMEOUT;
use test_utils::{create_service, timeout};

use eyre::{ContextCompat, WrapErr};
use futures::executor::block_on;
use itertools::Itertools;
use libp2p::core::Multiaddr;
use local_vm::read_args;
use log_utils::enable_logs;
use maplit::hashmap;
use serde::Deserialize;
use serde_json::json;
use serde_json::Value as JValue;
use service_modules::{load_module, module_config};
use std::str::FromStr;
use std::thread::sleep;
use std::time::{Duration, Instant};

fn permutations(swarms: &[CreatedSwarm]) -> Vec<Vec<String>> {
    use itertools::*;

    let pids = swarms.iter().map(|s| s.peer_id.to_string());
    let pids = pids.permutations(swarms.len()).collect();
    pids
}

#[test]
fn fold_fold_fold() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let nums: Vec<String> = (1..10).map(|i| i.to_string()).collect();
    let vec = vec![nums.clone(), nums.clone(), nums.clone()];
    let elems: Vec<(String, Vec<Vec<String>>)> = vec![
        ("a".into(), vec.clone()),
        ("a".into(), vec.clone()),
        ("a".into(), vec.clone()),
        ("a".into(), vec.clone()),
        ("a".into(), vec.clone()),
    ];

    client.send_particle(
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
                (fold $inner ns
                    (next ns)
                )
            )
            (seq
                (call relay ("op" "noop") [])
                (call client ("return" "") [$inner])
            )
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "permutations" => json!(elems),
        },
    );

    client.timeout = Duration::from_secs(1);

    let args = client.receive_args().wrap_err("receive args");
    if args.is_err() {
        panic!("{} failed", json!(elems));
    }
}

#[test]
fn fold_same_node_stream() {
    enable_logs();

    let swarms = make_swarms(4);

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

    let mut pid_permutations = permutations(&swarms).into_iter();
    let pid_permutations = &mut pid_permutations;
    let per_node = pid_permutations.len() / swarms.len();
    let permutations = swarms.iter().fold(vec![], |mut acc, swarm| {
        let perms = pid_permutations.take(per_node).collect::<Vec<_>>();
        assert_eq!(perms.len(), per_node);
        acc.push((swarm.peer_id.to_string(), perms));
        acc
    });

    client.send_particle(
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
                                        ;; (call pair.$.[0]! ("op" "noop") [])
                                        (null)
                                        (ap peer_ids $inner)
                                    )
                                    (next peer_ids)
                                )
                            )
                            (next pair)
                        )
                    )
                    (fold $inner ns
                    ;;    (seq
                    ;;        (fold ns n
                    ;;            (seq
                    ;;                (seq
                    ;;                    (call n ("op" "noop") [])
                    ;;                    (ap n $result)
                    ;;                )
                    ;;                (next n)
                    ;;            )
                    ;;        )
                            (next ns)
                    ;;    )
                    )
                )
            )
            (seq
                (call relay ("op" "noop") [])
                (call client ("return" "") [$inner])
            )
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "permutations" => json!(permutations),
        },
    );

    client.timeout = Duration::from_secs(1);

    let args = client.receive_args().wrap_err("receive args");
    if args.is_err() {
        panic!("{} failed", json!(permutations));
    }
}

#[test]
fn fold_same_node() {
    enable_logs();

    let swarms = make_swarms(4);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle(
        r#"
        (seq
            (fold node_arrays ns
                (seq
                    (fold ns n
                        (seq
                            (seq
                                (call n ("op" "noop") [])
                                (ap n $result)
                            )
                            (next n)
                        )
                    )
                    (next ns)
                )
            )
            (seq
                (call relay ("op" "noop") [])
                (call client ("return" "") [$result])
            )
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "node_arrays" => json!(permutations(&swarms))
        },
    );

    client.timeout = Duration::from_secs(120);

    client.receive_args().wrap_err("receive args").unwrap();
}
