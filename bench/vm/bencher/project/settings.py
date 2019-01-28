"""
Copyright 2018 Fluence Labs Limited

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
"""
from project.VMDescriptor import VMDescriptor
from project.TestDescriptor import TestDescriptor
from os.path import join

# launch count of interpreter-based VMs
interpreter_launches_count = 3

# launch count of compiler-based VMs
compiler_launches_count = 11

# export function name that should be called from each Wasm module
test_export_function_name = "main"

vm_descriptors = {
    "wavm"   : VMDescriptor(join("build_", "bin", "wavm-run"),
                            "{wasm_file_path} -f {function_name}", True),

    "life"   : VMDescriptor(join("life"), "-entry {function_name} {wasm_file_path}", False),

    "wasmi"  : VMDescriptor(join("target", "release", "examples", "invoke"),
                            "{wasm_file_path} {function_name}", False),

    "wasmer" : VMDescriptor(join("target", "release", "wasmer"), "run {wasm_file_path}", True),

    "wagon"  : VMDescriptor(join("cmd", "wasm-run"), "wasm_run {wasm_file_path}", False),

    "asmble" : VMDescriptor(join("asmble", "bin", "asmble"),
                            "invoke -in {wasm_file_path} {function_name} -defmaxmempages 20000", True)
}
