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

#![feature(try_blocks)]

use std::time::Duration;

use eyre::eyre;
use eyre::WrapErr;
use futures::channel::oneshot::channel;
use futures::executor::block_on;
use futures::FutureExt;
use maplit::hashmap;
use serde_json::json;
use serde_json::Value as JValue;

use connected_client::ConnectedClient;
use created_swarm::{make_swarms, CreatedSwarm};
use local_vm::{client_functions, wrap_script};
use log_utils::enable_logs;
use now_millis::now_ms;
use particle_protocol::Particle;
use test_constants::PARTICLE_TTL;
use uuid_utils::uuid;

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

    let pid_permutations = permutations(&swarms);
    let mut permutations = pid_permutations.iter();
    let permutations = &mut permutations;
    let per_node = permutations.len() / swarms.len();
    let permutations = swarms.iter().fold(vec![], |mut acc, swarm| {
        let perms = permutations.take(per_node).collect::<Vec<_>>();
        assert_eq!(perms.len(), per_node);
        acc.push((swarm.peer_id.to_string(), perms));
        acc
    });

    client.timeout = Duration::from_secs(200);
    client.particle_ttl = Duration::from_secs(200);

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
                                        (call pair.$.[0]! ("op" "noop") [])
                                        (ap peer_ids $inner)
                                    )
                                    (next peer_ids)
                                )
                            )
                            (next pair)
                        )
                    )
                    (fold $inner ns
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
                )
            )
            (seq
                (call relay ("op" "noop") [])
                (call client ("return" "") [$inner $result])
            )
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "permutations" => json!(permutations),
        },
    );

    let args = client.receive_args().wrap_err("receive args").unwrap();
    if let [JValue::Array(inner), JValue::Array(result)] = args.as_slice() {
        let inner: Vec<_> = inner
            .iter()
            .map(|a| {
                a.as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s.as_str().unwrap().to_string())
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(pid_permutations, inner);
        let flat: Vec<_> = pid_permutations.into_iter().flatten().collect();
        let result: Vec<_> = result
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        assert_eq!(flat, result);
    } else {
        panic!("expected 2 arrays");
    }
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

#[test]
fn fold_send_same_variable() {
    enable_logs();

    let swarms = make_swarms(5);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    println!("relay: {}", client.node);

    // let outer_iterable = vec!["1", "2", "3", "4", "5"];
    let outer_iterable: Vec<_> = swarms.iter().map(|s| s.peer_id.to_string()).collect();
    // let inner_iterable = vec![1, 2, 3, 4, 5];

    let mut data = hashmap! {
        "-relay-" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
        "outer" => json!(outer_iterable),

    };
    client.send_particle_ext(
        r#"
        (seq
            (seq
                (seq
                    (call %init_peer_id% ("getDataSrv" "-relay-") [] -relay-)
                    (call %init_peer_id% ("getDataSrv" "outer") [] outer)
                )
                (call -relay- ("op" "noop") [])
            )
            (fold outer o
                (par
                    (seq
                        (seq
                            (call -relay- ("op" "noop") [])
                            (call o ("op" "identity") [o] inner)
                        )
                        (par
                            (seq
                                (call -relay- ("op" "noop") [])
                                (call %init_peer_id% ("op" "return") [inner])
                            )
                            (null)
                        )
                    )
                    (next o)
                )
            )
        )
        "#,
        data,
        true,
    );

    client.timeout = Duration::from_secs(10);

    let mut iter = outer_iterable.len();
    loop {
        let args = client.receive_args().wrap_err("receive args");
        println!("args: {:?}", args);
        if args.is_err() {
            panic!("{:?} failed", args.err().unwrap());
        }
        iter = iter - 1;
        if iter == 0 {
            break;
        }
    }
}

#[test]
fn fold_dashboard() {
    enable_logs();

    let swarms = make_swarms(5);
    let sender = &swarms[0];

    let script = r#"
    (xor
 (seq
  (seq
   (call %init_peer_id% ("getDataSrv" "-relay-") [] -relay-)
   (call %init_peer_id% ("getDataSrv" "clientId") [] clientId)
  )
  (xor
   (par
    (seq
     (call -relay- ("kad" "neighborhood") [clientId [] []] neighbors)
     (call %init_peer_id% ("op" "noop") [])
    )
    (fold neighbors n
     (par
      (seq
       (call -relay- ("op" "noop") [])
       (xor
        (seq
         (seq
          (call n ("kad" "neighborhood") [n [] []] neighbors2)
          (par
           (seq
            (call -relay- ("op" "noop") [])
            (xor
             (call %init_peer_id% ("callbackSrv" "logNeighs") [neighbors2])
             (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 1])
            )
           )
           (null)
          )
         )
         (fold neighbors2 n2
          (seq
           (seq
            (seq
             (call -relay- ("op" "noop") [])
             (xor
              (seq
               (seq
                (seq
                 (seq
                  (seq
                   (call n2 ("peer" "identify") [] ident)
                   (call n2 ("dist" "list_blueprints") [] blueprints)
                  )
                  (call n2 ("dist" "list_modules") [] modules)
                 )
                 (call n2 ("srv" "list") [] services)
                )
                (call -relay- ("op" "noop") [])
               )
               (xor
                (call %init_peer_id% ("callbackSrv" "collectPeerInfo") [n2 ident services blueprints modules])
                (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 2])
               )
              )
              (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 3])
             )
            )
            (call -relay- ("op" "noop") [])
           )
           (next n2)
          )
         )
        )
        (seq
         (call -relay- ("op" "noop") [])
         (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 4])
        )
       )
      )
      (next n)
     )
    )
   )
   (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 5])
  )
 )
 (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 6])
)
    "#;
    let data = hashmap! {
        String::from("clientId") => json!(sender.peer_id.to_string()),
        String::from("-relay-") => json!(sender.peer_id.to_string())
    };
    let script = wrap_script(script.into(), &data, None, true, Some(sender.peer_id));
    let (outlet, inlet) = channel();

    let mut outlet = Some(outlet);
    let closure = move |args, _| {
        let result = client_functions(&data, args);

        if let Some(returned) = result.returned {
            if let Some(outlet) = outlet.take() {
                outlet.send(returned).expect("send response back")
            } else {
                // ignore further calls
            }
        }

        let outcome = result.outcome;
        async { outcome }.boxed()
    };

    let particle = Particle {
        id: uuid(),
        init_peer_id: sender.peer_id,
        timestamp: now_ms() as u64,
        ttl: PARTICLE_TTL,
        script,
        signature: vec![],
        data: vec![],
    };

    let aquamarine = sender.aquamarine_api.clone();
    let future = async move {
        try {
            aquamarine
                .execute(particle, Some(Box::new(closure)))
                .await?;

            let result = inlet.await;
            result
                .map_err(|err| eyre!("error reading from inlet: {:?}", err))?
                .map_err(|args| eyre!("AIR caught an error on args: {:?}", args))?
        }
    };

    let result: eyre::Result<Vec<JValue>> = block_on(future);
}
