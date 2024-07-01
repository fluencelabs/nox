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
