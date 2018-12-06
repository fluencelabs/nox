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
from VMDescriptor import VMDescriptor
from TestDescriptor import TestDescriptor
from os.path import join

# launch count of interpreter-based VMs
interpreter_launch_count = 3

# launch count of compiler-based VMs
compiler_launch_count = 11

# export function name that should be called from each Wasm module
test_function_name = "main"

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

test_descriptors = {
    # compressions tests are generated both for a small sequence with a lot of iterations
    # and for a huge sequence with few iterations
    "snappy_compression_5_1000000_1Kb" :
        TestDescriptor("compression",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED" : 1, "ITERATIONS_COUNT" : 1000000, "SEQUENCE_SIZE" : 1024}),

    "snappy_compression_5_10_100Mb" :
        TestDescriptor("compression",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED" : 1, "ITERATIONS_COUNT" : 10, "SEQUENCE_SIZE" : 100*1024*1024}),

    "deflate_compression_5_100000_1Kb" :
        TestDescriptor("compression",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown --features deflate_compression",
                       {"SEED": 1, "ITERATIONS_COUNT": 100000, "SEQUENCE_SIZE": 1024}),

    "deflate_compression_5_5_100Mb" :
        TestDescriptor("compression",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown --features deflate_compression",
                       {"SEED": 1, "ITERATIONS_COUNT": 5, "SEQUENCE_SIZE": 100 * 1024 * 1024}),

    "fibonacci_38" :
        TestDescriptor("fibonacci_bigint",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"FIB_NUMBER": 38}),

    "factorization_2147483647":
        TestDescriptor("factorization_reikna",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"FACTORIZED_NUMBER": 2147483647}),

    "recursive_hash_10000000_0":
        TestDescriptor("recursive_hash",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"ITERATIONS_COUNT": 10000000, "INITIAL_VALUE" : 0}),

    # matrix tests are generated both for a small matrix with a lot of iterations
    # and for a huge matrix with few iterations
    "matrix_product_1_10_1000000":
        TestDescriptor("matrix_product",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "MATRIX_SIZE": 10, "ITERATIONS_COUNT" : 1000000}),

    "matrix_product_1_500_100":
        TestDescriptor("matrix_product",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "MATRIX_SIZE": 500, "ITERATIONS_COUNT": 100}),

    "svd_decomposition_1_10_1000000":
        TestDescriptor("matrix_svd_decomposition",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "MATRIX_SIZE": 10, "ITERATIONS_COUNT" : 1000000}),

    "svd_decomposition_1_300_100":
        TestDescriptor("matrix_svd_decomposition",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "MATRIX_SIZE": 300, "ITERATIONS_COUNT": 100}),

    "qr_decomposition_1_10_1000000":
        TestDescriptor("matrix_qr_decomposition",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "MATRIX_SIZE": 10, "ITERATIONS_COUNT": 1000000}),

    "qr_decomposition_1_500_100":
        TestDescriptor("matrix_qr_decomposition",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "MATRIX_SIZE": 500, "ITERATIONS_COUNT": 100})
}
