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

use eyre::{Result, WrapErr};
use std::path::{Path, PathBuf};

pub fn air_interpreter_path(base_dir: &Path) -> PathBuf {
    use air_interpreter_wasm as interpreter;

    base_dir.join(format!("aquamarine_{}.wasm", interpreter::VERSION))
}

pub fn write_default_air_interpreter(destination: &Path) -> Result<()> {
    use air_interpreter_wasm::INTERPRETER_WASM;
    use std::fs::write;

    write(destination, INTERPRETER_WASM).wrap_err(format!(
        "failed writing default INTERPRETER_WASM to {:?}",
        destination
    ))
}
