/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

#![feature(assert_matches)]

use eyre::WrapErr;
use maplit::hashmap;
use serde_json::Value as JValue;
use serde_json::{json, Value};
use std::assert_matches::assert_matches;

use base64::{engine::general_purpose::STANDARD as base64, Engine};
use connected_client::ConnectedClient;
use created_swarm::{make_swarms, make_swarms_with_cfg};
use jsonrpsee::core::async_trait;
use service_modules::{load_module, Hash};
use system_services::{CallService, Deployment, InitService, PackageDistro, ServiceDistro};

#[tokio::test]
async fn test_system_service_override() {
    // We need to include bytes, not read them, since the ServiceDistro expects module bytes as `&'static [u8]`
    // It's unnecessary to allow not static links or even vectors since real life we need only this
    let module = include_bytes!("./tetraplets/artifacts/tetraplets.wasm");
    let config = json!({
      "total_memory_limit": "Infinity",
      "module": [
        {
          "name": "tetraplets",
          "config" : {
            "preopened_files": [
               [
                 "tmp"
               ]
            ],
        }}
      ]
    });

    let config = serde_json::from_value(config).expect("parse module config");
    let m: &'static [u8] = module;
    let service_name = "test-service".to_string();

    let service = ServiceDistro {
        name: service_name.clone(),
        modules: hashmap! {
            "tetraplets" => m,
        },
        config,
    };
    let name = service_name.clone();

    struct Test {
        name: String,
    }
    #[async_trait]
    impl InitService for Test {
        async fn init(
            &self,
            call_service: &dyn CallService,
            deployment: Deployment,
        ) -> eyre::Result<()> {
            let name = self.name.clone();
            let service_status = deployment
                .services
                .get(&name)
                .expect("deployment status for the service");
            assert_matches!(
                service_status,
                system_services::ServiceStatus::Created(_),
                "wrong deployment status"
            );
            let result: eyre::Result<_> = call_service
                .call(name.clone(), "not".to_string(), vec![json!(false)])
                .await;
            assert!(
                result.is_err(),
                "must be error due to the the call interface restrictions"
            );
            let error = result.unwrap_err().to_string();
            assert_eq!(
                error,
                format!("Call {}.not return invalid result: true", name),
                "the service call must return invalid result due to the call interface restrictions"
            );
            Ok(())
        }
    }

    let init = Box::new(Test { name });
    let package = PackageDistro {
        name: service_name.clone(),
        version: "some-version",
        services: vec![service],
        spells: vec![],
        init: Some(std::sync::Arc::new(init)),
    };
    let swarms = make_swarms_with_cfg(1, move |mut cfg| {
        cfg.extend_system_services = vec![package.clone()];
        cfg
    })
    .await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();
    let data = hashmap! {
        "relay" => json!(client.node.to_string()),
    };
    let response = client
        .execute_particle(
            r#"
                (seq
                    (call relay ("srv" "list") [] list)
                    (call %init_peer_id% ("return" "") [list])
                ) 
            "#,
            data,
        )
        .await
        .unwrap();
    if let [Value::Array(list)] = response.as_slice() {
        assert_eq!(list.len(), 1, "expected only one service to be installed");
        if let Value::Object(obj) = &list[0] {
            let aliases = obj
                .get("aliases")
                .expect("srv.list must return a list of aliases for a service")
                .as_array()
                .expect("list of aliases must be a list");
            assert_eq!(aliases.len(), 1, "test-service must have only 1 alias");
            assert_eq!(aliases[0], service_name, "wrong alias for the test-service");
        }
    } else {
        panic!("wrong result, expected list of services")
    }
}

#[tokio::test]
async fn create_service_from_config() {
    let swarms = make_swarms_with_cfg(1, move |mut cfg| {
        cfg.allowed_binaries = vec!["/does/not/exist".into()];
        cfg
    })
    .await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
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
                "/does/not/exist"
              ],
              [
                "2222",
                "/does/not/exist"
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
                 (ap mod $mod_hashes)
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
    client.send_particle_ext(script, data, true).await;
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

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
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
    client.send_particle_ext(script, data, true).await;
    let result = client.receive_args().await;
    if result.is_ok() {
        panic!("expected error for module with invalid config")
    }
}

#[tokio::test]
async fn test_create_service_by_other_forbidden() {
    let swarms = make_swarms(1).await;
    let mut client_manager = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();
    let module_name = "tetraplets";
    let module_bytes = load_module("tests/tetraplets/artifacts", module_name).expect("load module");
    let response = client_manager
        .execute_particle(
            r#"
        (seq
            (seq
                (seq
                    (call relay ("dist" "default_module_config") [module_name] module_config)
                    (call relay ("dist" "add_module") [module_bytes module_config] module)
                )
                (seq
                    (call relay ("dist" "make_blueprint") [name dependencies] blueprint)
                    (call relay ("dist" "add_blueprint") [blueprint] blueprint_id)
                )
            )
            (call client ("return" "") [blueprint_id])
        )"#,
            hashmap! {
               "client" => json!(client_manager.peer_id.to_string()),
                "relay" => json!(client_manager.node.to_string()),
                "module_name" => json!("module"),
                "dependencies" => json!([Hash::new(&module_bytes).unwrap()]),
                "module_bytes" => json!(base64.encode(module_bytes)),
                "name" => json!("service1"),
            },
        )
        .await
        .unwrap();
    let blueprint_id = response[0]
        .as_str()
        .expect("blueprint_id is in response")
        .to_string();
    let mut client_other = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();
    let response = client_other
        .execute_particle(
            r#"
        (xor
            (seq
                (call relay ("srv" "create") [blueprint_id] service_id)
                (call client ("return" "") ["ok"])
            )
            (call client ("return" "") ["forbidden"])
        )
        "#,
            hashmap! {
                "client" => json!(client_other.peer_id.to_string()),
                "relay" => json!(client_other.node.to_string()),
                "blueprint_id" => json!(blueprint_id),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        "forbidden",
        response[0].as_str().unwrap(),
        "got {:?}",
        response[0]
    );
}
