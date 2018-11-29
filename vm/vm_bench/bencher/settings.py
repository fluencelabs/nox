from VMDescriptor import VMDescriptor
from os.path import join

# paths of Wasm VMs root directories
vm_descriptors = {
    "wavm"   : VMDescriptor("wavm", join("build_", "bin", "wavm-run"),
                            "{wasm_file_path} -f {function_name}", True),
    "life"   : VMDescriptor("life", join("life"), "entry '{function_name}' {wasm_file_path}", False),
    "wasmi"  : VMDescriptor("wasmi", join("target", "release", "examples", "invoke"),
                            "{function_name} {wasm_file_path}", False),
    "wasmer" : VMDescriptor("wasmer", join("target", "release", "wasmer"), "run {wasm_file_path}", True),
    "wagon"  : VMDescriptor("wagon", "", "", False),
    "asmble" : VMDescriptor("asmble", join("asmble", "bin", "asmble"),
                            "invoke -in {wasm_file_path} {function_name} -defmaxmempages 20000", True)
}

# launch count of interpretator-based VMs
interpretator_launch_count = 0

# launch count of compiler-based VMs
compiler_launch_count = 1

# export function name that should be called from Wasm module
test_function_name = "main"

# paths of benchmark tests
snappy_compression_test_name    = "snappy_compression"
deflate_compression_test_name   = "deflate_compression"
fibonacci_bigint_test_name      = "fibonacci_bigint"
factorization_test_name         = "factorization_reikna"
recursive_hash_test_name        = "recursive_hash"
matrix_product_test_name        = "matrix_product"
qr_decompostion_test_name       = "matrix_qr_decomposition"
svd_decompostion_test_name      = "matrix_svd_decomposition"
