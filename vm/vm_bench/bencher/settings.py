from VMDescriptor import VMDescriptor
from TestDescriptor import TestDescriptor
from os.path import join

# paths of Wasm VMs root directories
vm_descriptors = {
    "wavm"   : VMDescriptor(join("build_", "bin", "wavm-run"),
                            "{wasm_file_path} -f {function_name}", True),
    "life"   : VMDescriptor(join("life"), "entry '{function_name}' {wasm_file_path}", False),
    "wasmi"  : VMDescriptor(join("target", "release", "examples", "invoke"),
                            "{function_name} {wasm_file_path}", False),
    "wasmer" : VMDescriptor(join("target", "release", "wasmer"), "run {wasm_file_path}", True),
    "wagon"  : VMDescriptor("", "", False),
    "asmble" : VMDescriptor(join("asmble", "bin", "asmble"),
                            "invoke -in {wasm_file_path} {function_name} -defmaxmempages 20000", True)
}

# launch count of interpretator-based VMs
interpretator_launch_count = 0

# launch count of compiler-based VMs
compiler_launch_count = 11

# export function name that should be called from Wasm module
test_function_name = "main"

# paths of benchmark tests
test_descriptors = {
    # compressions tests are generated both for a small sequence with a lot of iterations
    # and for a huge sequence with a few iterations
    "snappy_compression_5_1000000_1Kb" :
        TestDescriptor("snappy_compression",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED" : 1, "ITERATIONS_COUNT" : 1000000, "SEQUENCE_SIZE" : 1024}),

    "snappy_compression_5_1000_10Mb" :
        TestDescriptor("snappy_compression",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED" : 1, "ITERATIONS_COUNT" : 1000, "SEQUENCE_SIZE" : 10*1024*1024}),

    "deflate_compression_5_100000_1Kb" :
        TestDescriptor("deflate_compression",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "ITERATIONS_COUNT": 100000, "SEQUENCE_SIZE": 1024}),

    "deflate_compression_5_10_10Mb" :
        TestDescriptor("deflate_compression",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "ITERATIONS_COUNT": 10, "SEQUENCE_SIZE": 10 * 1024 * 1024}),

    "fibonacci_42" :
        TestDescriptor("fibonacci_bigint",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"FIB_NUMBER": 42}),

    "fibonacci_50" :
        TestDescriptor("fibonacci_bigint",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"FIB_NUMBER": 50}),

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
    # and for a huge matrix with a few iterations
    "matrix_product_1_10_1000000":
        TestDescriptor("matrix_product",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "MATRIX_SIZE": 10, "ITERATIONS_COUNT" : 1000000}),

    "matrix_product_1_200_100":
        TestDescriptor("matrix_product",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "MATRIX_SIZE": 200, "ITERATIONS_COUNT": 100}),

    "svd_decomposition_1_10_1000000":
        TestDescriptor("matrix_svd_decomposition",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "MATRIX_SIZE": 10, "ITERATIONS_COUNT" : 1000000}),

    "svd_decomposition_1_200_100":
        TestDescriptor("matrix_svd_decomposition",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "MATRIX_SIZE": 200, "ITERATIONS_COUNT": 100}),

    "qr_decomposition_1_10_1000000":
        TestDescriptor("matrix_qr_decomposition",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "MATRIX_SIZE": 10, "ITERATIONS_COUNT": 1000000}),

    "qr_decomposition_1_200_100":
        TestDescriptor("matrix_qr_decomposition",
                       "cargo build --manifest-path {}/Cargo.toml --target-dir {} --release "
                       "--target wasm32-unknown-unknown",
                       {"SEED": 1, "MATRIX_SIZE": 200, "ITERATIONS_COUNT": 100})
}
