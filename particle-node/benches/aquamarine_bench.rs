// /*
//  * Copyright 2020 Fluence Labs Limited
//  *
//  * Licensed under the Apache License, Version 2.0 (the "License");
//  * you may not use this file except in compliance with the License.
//  * You may obtain a copy of the License at
//  *
//  *     http://www.apache.org/licenses/LICENSE-2.0
//  *
//  * Unless required by applicable law or agreed to in writing, software
//  * distributed under the License is distributed on an "AS IS" BASIS,
//  * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  * See the License for the specific language governing permissions and
//  * limitations under the License.
//  */
//
// use criterion::Criterion;
// use criterion::{criterion_group, criterion_main};
// use fluence_libp2p::RandomPeerId;
// use local_vm::{make_call_service_closure, make_vm};
// use maplit::hashmap;
// use parking_lot::Mutex;
// use serde_json::{json, Value as JValue};
// use std::collections::HashMap;
// use std::sync::Arc;
//
// fn bench_script(name: &str, script: &str, data: HashMap<&'static str, JValue>, c: &mut Criterion) {
//     let peer_id = RandomPeerId::random();
//
//     let data: HashMap<_, _> = data.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
//     let data = Arc::new(Mutex::new(data));
//
//     let mut vm = make_vm(peer_id);
//
//     // warm
//     vm.call(peer_id.to_base58(), script, vec![], "id").unwrap();
//
//     c.bench_function(name, move |b| {
//         b.iter(|| vm.call(peer_id.to_base58(), script, vec![], "id").unwrap())
//     });
// }
//
// fn next_node_bench(c: &mut Criterion) {
//     let script = r#"(call "other" ("op" "identity") [])"#;
//     bench_script("next_node", script, hashmap! {}, c)
// }
//
// fn next_node_with_data_bench(c: &mut Criterion) {
//     let script = r#"(call "other" ("op" "identity") ["data"] result)"#;
//     bench_script("next_node_with_data", script, hashmap! {}, c)
// }
//
// fn host_call(c: &mut Criterion) {
//     let script = r#"
//     (call %init_peer_id% ("op" "identity") ["data"] result)
//     "#;
//
//     bench_script("host_call", script, hashmap! {}, c)
// }
//
// fn host_load(c: &mut Criterion) {
//     let script = r#"
//     (call %init_peer_id% ("load" "data") [data] result)
//     "#;
//
//     bench_script("host_load", script, hashmap! { "data" => json!("data") }, c)
// }
//
// fn host_load_return(c: &mut Criterion) {
//     let script = r#"
//     (seq
//         (call %init_peer_id% ("load" "data") [data] result)
//         (call %init_peer_id% ("op" "return") [data])
//     )
//     "#;
//
//     bench_script(
//         "host_load_return",
//         script,
//         hashmap! { "data" => json!("data") },
//         c,
//     )
// }
//
// criterion_group!(
//     benches,
//     next_node_bench,
//     next_node_with_data_bench,
//     host_call,
//     host_load,
//     host_load_return,
// );
// criterion_main!(benches);

fn main() {}
