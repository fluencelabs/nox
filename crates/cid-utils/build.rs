/*
 * Copyright 2024 Fluence DAO
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

use pb_rs::{types::FileDescriptor, ConfigBuilder};
use std::path::Path;

fn main() {
    let unixfs_proto_path = Path::new("src/unixfs.proto");
    // Re-run this build.rs if the proto changes
    println!(
        "cargo:rerun-if-changed={}",
        unixfs_proto_path.to_str().expect("valid .proto path")
    );

    let config_builder = ConfigBuilder::new(
        &[unixfs_proto_path],
        None,
        None,
        &[unixfs_proto_path.parent().unwrap()],
    )
    .expect("create config builder for rs generation");

    // generate rs from proto
    FileDescriptor::run(&config_builder.single_module(true).build())
        .expect("generate rs from proto");

    // pb_rs generates mod.rs, but we don't need it and there is no way to turn it off
    std::fs::remove_file("src/mod.rs").unwrap();
}
