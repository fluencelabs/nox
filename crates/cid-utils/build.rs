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

use pb_rs::{types::FileDescriptor, ConfigBuilder};
use std::path::{Path, PathBuf};

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let in_dir = PathBuf::from(::std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("src");
    let unixfs_proto_path = in_dir.join("unixfs.proto"); //PathBuf::from(::std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/unixfs.proto");

    // Re-run this build.rs if the protos dir changes (i.e. a new file is added)
    println!("cargo:rerun-if-changed={}", unixfs_proto_path.to_str().expect("valid .proto path"));

    // Delete all old generated files before re-generating new ones
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).unwrap();
    }
    std::fs::DirBuilder::new().create(&out_dir).unwrap();
    let config_builder = ConfigBuilder::new(&[unixfs_proto_path], None, Some(&out_dir.to_path_buf()), &[in_dir]).unwrap();
    FileDescriptor::run(&config_builder.build()).unwrap()
}
