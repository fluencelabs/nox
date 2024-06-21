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

// use jsonrpsee::core::JsonValue;
// use jsonrpsee::server::Server;
// use jsonrpsee::{RpcModule, SubscriptionMessage};
// use tokio::task::JoinHandle;

// async fn run_server() -> eyre::Result<(String, JoinHandle<()>)> {
//     let server = Server::builder()
//         .set_message_buffer_capacity(10)
//         .build("127.0.0.1:0")
//         .await?;
//     let mut module = RpcModule::new(());
//     module
//         .register_subscription(
//             "eth_subscribe",
//             "eth_subscription",
//             "eth_unsubscribe",
//             |params, pending, _ctx| async move {
//                 let cc_event_log: JsonValue = serde_json::from_str(r#"
//                         {
//                           "address": "0xdc64a140aa3e981100a9beca4e685f962f0cf6c9",
//                           "topics": [
//                             "0xcd92fc03744bba25ad966bdc1127f8996e70c551d1ee4a88ce7fb0e596069649",
//                             "0x246cd65bc58db104674f76c9b1340eb16881d9ef90e33d4b1086ebd334f4002d",
//                             "0xd6996a1d0950671fa4ae2642e9bfdb7e4c7832a35c640cdb47ecb8b8002b77b5"
//                           ],
//                           "data": "0x00000000000000000000000000000000000000000000000000000000009896800000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000a4c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc9",
//                           "blockHash": "0x85f298013ad4ea98d279ef543d8efc3aac0563aabbb623567f7146e9a24aa000",
//                           "blockNumber": "0x6",
//                           "transactionHash": "0xcad9a9e67e1503513973b2c73052bb64b2709ea4578b04e0464624d7a2f92e61",
//                           "transactionIndex": "0x0",
//                           "logIndex": "0x0",
//                           "transactionLogIndex": "0x0",
//                           "removed": false
//                         }"#).unwrap();
//
//                 let new_heads: JsonValue = serde_json::from_str( r#"
//                         {
//                           "difficulty": "0x15d9223a23aa",
//                           "extraData": "0xd983010305844765746887676f312e342e328777696e646f7773",
//                           "gasLimit": "0x47e7c4",
//                           "gasUsed": "0x38658",
//                           "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
//                           "miner": "0xf8b483dba2c3b7176a3da549ad41a48bb3121069",
//                           "nonce": "0x084149998194cc5f",
//                           "number": "0x1348c9",
//                           "parentHash": "0x7736fab79e05dc611604d22470dadad26f56fe494421b5b333de816ce1f25701",
//                           "receiptRoot": "0x2fab35823ad00c7bb388595cb46652fe7886e00660a01e867824d3dceb1c8d36",
//                           "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
//                           "stateRoot": "0xb3346685172db67de536d8765c43c31009d0eb3bd9c501c9be3229203f15f378",
//                           "timestamp": "0x56ffeff8",
//                           "transactionsRoot": "0x0167ffa60e3ebc0b080cdb95f7c0087dd6c0e61413140e39d94d3468d7c9689f"
//                         }"#).unwrap();
//                 if let Ok(sub) = params.sequence().next::<String>() {
//                     let sink = pending.accept().await.unwrap();
//
//                     if sub == "newHeads" {
//                         sink.send(SubscriptionMessage::from_json(&new_heads).unwrap())
//                             .await.unwrap();
//                     }
//
//                     if sub == "logs" {
//                         sink.send(SubscriptionMessage::from_json(&cc_event_log).unwrap())
//                             .await.unwrap();
//                     }
//                 }
//             },
//         )
//         .unwrap();
//
//     let addr = server.local_addr()?;
//     let handle = server.start(module);
//
//     let handle = tokio::spawn(handle.stopped());
//     Ok((addr.to_string(), handle))
// }
//
// #[tokio::test]
// async fn test_chain_listener_cc() {
//     let (addr, server) = run_server().await.unwrap();
//     let url = format!("ws://{}", addr);
//     let events_dir = TempDir::new().unwrap();
//     let cc_events_dir = events_dir.path().to_path_buf();
//     let _swarm = make_swarms_with_cfg(1, move |mut cfg| {
//         cfg.chain_listener = Some(ChainConfig {
//             ws_endpoint: url.clone(),
//             http_endpoint: "".to_string(),
//             cc_contract_address: "".to_string(),
//             core_contract_address: "".to_string(),
//             market_contract_address: "".to_string(),
//             network_id: 0,
//             wallet_key: PrivateKey::from_str(
//                 "0xfdc4ba94809c7930fe4676b7d845cbf8fa5c1beae8744d959530e5073004cf3f",
//             )
//             .unwrap(),
//             ccp_endpoint: "".to_string(),
//             timer_resolution: Default::default(),
//         });
//
//         cfg.cc_events_dir = Some(cc_events_dir.clone());
//         cfg
//     })
//     .await;
//
//     tokio::time::sleep(std::time::Duration::from_millis(200)).await;
//
//     let event_files = list_files(events_dir.path()).unwrap().collect::<Vec<_>>();
//     assert_eq!(event_files.len(), 1);
//
//     server.abort();
// }
