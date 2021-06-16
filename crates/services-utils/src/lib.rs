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

use fs_utils::{create_dir, to_abs_path};
use std::path::PathBuf;

pub type Result<T> = eyre::Result<T>;

pub fn put_aquamarine(tmp: PathBuf) -> PathBuf {
    use air_interpreter_wasm::{INTERPRETER_WASM, VERSION};

    create_dir(&tmp).expect("create tmp dir");

    let file = to_abs_path(tmp.join(format!("aquamarine_{}.wasm", VERSION)));
    std::fs::write(&file, INTERPRETER_WASM)
        .unwrap_or_else(|_| panic!("fs::write aquamarine.wasm to {:?}", file));

    file
}

pub fn load_module(path: &str, module_name: &str) -> Vec<u8> {
    let module = to_abs_path(PathBuf::from(path).join(format!("{}.wasm", module_name)));
    std::fs::read(&module).unwrap_or_else(|_| panic!("fs::read from {:?}", module))
}
