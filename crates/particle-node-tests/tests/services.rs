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

use eyre::WrapErr;
use maplit::hashmap;
use serde_json::json;
use serde_json::Value as JValue;

use base64::{engine::general_purpose::STANDARD as base64, Engine};
use connected_client::ConnectedClient;
use created_swarm::make_swarms;
use service_modules::load_module;

#[tokio::test]
async fn create_service_from_config() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let module = load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module");

    let config = json!({
      "modules": [
        {
          "name": "pure_base64",
          "preopened_files": [
            [
              "tmp"
            ]
          ],
          "mem_pages_count": [
            123
          ],
          "mapped_dirs": [],
          "logger_enabled": [
            true
          ],
          "max_heap_size": [
            "100 Mib"
          ],
          "logging_mask": [
            4
          ],
          "envs": [
            []
          ],
          "mounted_binaries": [
            [
              [
                "abc",
                "/tmp"
              ],
              [
                "2222",
                "/tmp"
              ]
            ]
          ]
        }
      ]
    });

    let script = r#"
(xor
 (seq
  (seq
   (seq
    (seq
     (seq
      (call %init_peer_id% ("getDataSrv" "-relay-") [] -relay-)
      (seq
        (call %init_peer_id% ("getDataSrv" "config") [] config)
        (call %init_peer_id% ("getDataSrv" "module_bytes") [] module_bytes)
      )
     )
     (new $mod_hashes
      (seq
       (seq
        (seq
         (seq
          (seq
           (seq
            (null)
            (fold config.$.modules! m-0
             (seq
              (seq
               (null) ;; uploading was here
               (xor
                (seq
                 (seq
                  ;; downloading was here
                  (call -relay- ("dist" "make_module_config") [m-0.$.name! m-0.$.mem_pages_count! m-0.$.max_heap_size! m-0.$.logger_enabled! m-0.$.preopened_files! m-0.$.envs! m-0.$.mapped_dirs! m-0.$.mounted_binaries! m-0.$.logging_mask!] conf)
                  (call -relay- ("dist" "add_module") [module_bytes conf] mod)
                 )
                 (call -relay- ("op" "concat_strings") ["hash:" mod] $mod_hashes)
                )
                (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 2])
               )
              )
              (next m-0)
             )
            )
           )
           (null)
          )
          (xor
           (seq
            (seq
             (seq
              (canon -relay- $mod_hashes #mod_hashes)
              (call -relay- ("dist" "make_blueprint") ["pure_base64" #mod_hashes] blueprint)
             )
             (call -relay- ("dist" "add_blueprint") [blueprint] blueprint_id)
            )
            (call -relay- ("srv" "create") [blueprint_id] service_id)
           )
           (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 3])
          )
         )
         (null)
        )
        (null)
       )
       (null)
      )
     )
    )
    (null)
   )
   (null)
  )
  (xor
   (call %init_peer_id% ("callbackSrv" "response") [service_id])
   (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 4])
  )
 )
 (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 5])
)
    "#;

    let data = hashmap! {
        "-relay-" => json!(client.node.to_string()),
        "config" => config,
        "module_bytes" => json!(base64.encode(module)),
    };
    client.send_particle_ext(script, data, true);
    let result = client.receive_args().await.expect("receive");
    if let [JValue::String(service_id)] = &result[..] {
        let result = client
            .execute_particle(
                r#"
            (seq
                (call relay ("srv" "list") [] list)
                (call %init_peer_id% ("op" "return") [list])
            )
            "#,
                hashmap! {
                    "relay" => json!(client.node.to_string()),
                    "service" => json!(service_id),
                },
            )
            .await
            .unwrap();

        use serde_json::Value::Array;

        if let [Array(sids)] = result.as_slice() {
            let sid = sids.first().unwrap().get("id").unwrap();
            assert_eq!(sid, &json!(service_id))
        } else {
            panic!("incorrect args: expected vec of single string")
        }
    }
}

#[tokio::test]
async fn handle_same_dir_in_preopens_and_mapped_dirs() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let module = load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module");

    let config = json!({
      "modules": [
        {
          "name": "pure_base64",
          "preopened_files": [
            [
              "/tmp"
            ]
          ],
          "mem_pages_count": [
            123
          ],
          "mapped_dirs": [
            ["/tmp", "/tmp"]
        ],
          "logger_enabled": [
            true
          ],
          "max_heap_size": [
            "100 Mib"
          ],
          "logging_mask": [
            4
          ],
          "envs": [
            []
          ],
          "mounted_binaries": [
            [
              [
                "abc",
                "/tmp"
              ],
              [
                "2222",
                "/tmp"
              ]
            ]
          ]
        }
      ]
    });

    let script = r#"
(xor
 (seq
  (seq
   (seq
    (seq
     (seq
      (call %init_peer_id% ("getDataSrv" "-relay-") [] -relay-)
      (seq
        (call %init_peer_id% ("getDataSrv" "config") [] config)
        (call %init_peer_id% ("getDataSrv" "module_bytes") [] module_bytes)
      )
     )
     (new $mod_hashes
      (seq
       (seq
        (seq
         (seq
          (seq
           (seq
            (null)
            (fold config.$.modules! m-0
             (seq
              (seq
               (null) ;; uploading was here
               (xor
                (seq
                 (seq
                  ;; downloading was here
                  (call -relay- ("dist" "make_module_config") [m-0.$.name! m-0.$.mem_pages_count! m-0.$.max_heap_size! m-0.$.logger_enabled! m-0.$.preopened_files! m-0.$.envs! m-0.$.mapped_dirs! m-0.$.mounted_binaries! m-0.$.logging_mask!] conf)
                  (call -relay- ("dist" "add_module") [module_bytes conf] mod)
                 )
                 (call -relay- ("op" "concat_strings") ["hash:" mod] $mod_hashes)
                )
                (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 2])
               )
              )
              (next m-0)
             )
            )
           )
           (null)
          )
          (xor
           (seq
            (seq
             (seq
              (canon -relay- $mod_hashes #mod_hashes)
              (call -relay- ("dist" "make_blueprint") ["pure_base64" #mod_hashes] blueprint)
             )
             (call -relay- ("dist" "add_blueprint") [blueprint] blueprint_id)
            )
            (call -relay- ("srv" "create") [blueprint_id] service_id)
           )
           (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 3])
          )
         )
         (null)
        )
        (null)
       )
       (null)
      )
     )
    )
    (null)
   )
   (null)
  )
  (xor
   (call %init_peer_id% ("callbackSrv" "response") [service_id])
   (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 4])
  )
 )
 (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 5])
)
    "#;

    let data = hashmap! {
        "-relay-" => json!(client.node.to_string()),
        "config" => config,
        "module_bytes" => json!(base64.encode(module)),
    };
    client.send_particle_ext(script, data, true);
    let result = client.receive_args().await;
    if result.is_ok() {
        panic!("expected error for module with invalid config")
    }
}
