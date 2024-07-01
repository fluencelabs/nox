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
        "failed writing default INTERPRETER_WASM to {destination:?}"
    ))
}
