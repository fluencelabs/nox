/*
 * Copyright 2018 Fluence Labs Limited
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

use std::process::Command;

fn main() {
    println!("Installing npm...");

    let mut install_cmd = Command::new("npm");
    install_cmd.current_dir("../bootstrap/");
    install_cmd.args(&["install"]);
    install_cmd.status().unwrap();

    println!("Compiling sol files...");

    let mut gen_cmd = Command::new("npm");
    gen_cmd.current_dir("../bootstrap/");
    gen_cmd.args(&["run", "compile-sol"]);
    gen_cmd.status().unwrap();
}
