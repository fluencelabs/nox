from os.path import join
from settings import *
from subprocess import call


class BenchTestGenerator:

    def __init__(self, test_dir):
        self.tests_dir = test_dir
        self.generated_tests_dir = "bench_tests"
        self.rename_cmd = ""

    # compiles test with chosen parameters
    def generate_tests(self, out_dir):
        snappy_full_path = join(self.tests_dir, snappy_compression_test_name)
        deflate_full_path = join(self.tests_dir, deflate_compression_test_name)
        fibonacci_full_path = join(self.tests_dir, fibonacci_bigint_test_name)
        hash_full_path = join(self.tests_dir, recursive_hash_test_name)
        factorization_full_path = join(self.tests_dir, factorization_test_name)
        product_full_path = join(self.tests_dir, matrix_product_test_name)
        qr_full_path = join(self.tests_dir, qr_decompostion_test_name)
        svd_full_path = join(self.tests_dir, svd_decompostion_test_name)

        call("mkdir {}".format(join(out_dir,self.generated_tests_dir)),shell=True)

        self.rename_cmd = "mv " + join(out_dir, "wasm32-unknown-unknown", "release", "{}.wasm") + " " \
                          + join(out_dir, self.generated_tests_dir, "{}.wasm")

        generated_test_paths = []

        # compressions tests are generated both for a small sequence with a lot of iterations
        # and for a huge sequence with a few iterations
        generated_test_paths.append(self.__compile_compression_test(5, 1000000, 1024, snappy_full_path, out_dir,
                                                                    snappy_compression_test_name))
        generated_test_paths.append(self.__compile_compression_test(5, 1000, 10*1024*1024, snappy_full_path, out_dir,
                                                                    snappy_compression_test_name))

        generated_test_paths.append(self.__compile_compression_test(5, 1000000, 1024, deflate_full_path, out_dir,
                                                                    deflate_compression_test_name))
        generated_test_paths.append(self.__compile_compression_test(5, 1000, 10*1024*1024, deflate_full_path, out_dir,
                                                                    deflate_compression_test_name))

        generated_test_paths.append(self.__compile_fibonacci_test(47, fibonacci_full_path, out_dir))

        generated_test_paths.append(self.__compile_hash_test(1000000, 0, hash_full_path, out_dir))

        generated_test_paths.append(self.__compile_factorization_test(1000000, factorization_full_path, out_dir))

        # matrix tests are generated both for a small matrix with a lot of iterations
        # and for a huge matrix with a few iterations
        generated_test_paths.append(self.__compile_matrix_test(1, 10, 1000000, product_full_path, out_dir,
                                                               matrix_product_test_name))
        generated_test_paths.append(self.__compile_matrix_test(1, 200, 100, product_full_path, out_dir,
                                                               matrix_product_test_name))

        generated_test_paths.append(self.__compile_matrix_test(1, 10, 1000000, qr_full_path, out_dir,
                                                               qr_decompostion_test_name))
        generated_test_paths.append(self.__compile_matrix_test(1, 200, 100, qr_full_path, out_dir,
                                                               qr_decompostion_test_name))

        generated_test_paths.append(self.__compile_matrix_test(1, 10, 1000000, svd_full_path, out_dir,
                                                               svd_decompostion_test_name))
        generated_test_paths.append(self.__compile_matrix_test(1, 200, 100, svd_full_path, out_dir,
                                                               svd_decompostion_test_name))

        # collect garbage
        call(("rm -rf " + join("{}", "wasm32-unknown-unknown")).format(out_dir), shell=True)
        call(("rm -rf " + join("{}", "release")).format(out_dir), shell=True)

        return generated_test_paths

    def __compile_compression_test(self, seed, iterations_count, sequence_size, test_full_path, out_dir,
                                   compression_test_name):
        compression_compile_string = "SEED={} ITERATIONS_COUNT={} SEQUENCE_SIZE={} cargo build --manifest-path {}/Cargo.toml " \
                                "--target-dir {} --release --target wasm32-unknown-unknown"
        call(compression_compile_string.format(seed, iterations_count, sequence_size, test_full_path, out_dir),
             shell=True)

        test_name = f"{compression_test_name}_{seed}_{iterations_count}_{sequence_size}"
        call(self.rename_cmd.format(compression_test_name, test_name), shell=True)
        return join(out_dir, self.generated_tests_dir, test_name) + ".wasm"

    def __compile_fibonacci_test(self, fib_number, test_full_path, out_dir):
        fibonacci_compile_string = "FIB_NUMBER={} cargo build --manifest-path {}/Cargo.toml " \
                                "--target-dir {} --release --target wasm32-unknown-unknown"
        call(fibonacci_compile_string.format(fib_number, test_full_path, out_dir), shell=True)

        test_name = f"{fibonacci_bigint_test_name}_{fib_number}"
        call(self.rename_cmd.format(fibonacci_bigint_test_name, test_name), shell=True)
        return join(out_dir, self.generated_tests_dir, test_name) + ".wasm"

    def __compile_factorization_test(self, factorized_number, test_full_path, out_dir):
        factorization_compile_string = "FACTORIZED_NUMBER={} cargo build --manifest-path {}/Cargo.toml " \
                                "--target-dir {} --release --target wasm32-unknown-unknown"
        call(factorization_compile_string.format(factorized_number, test_full_path, out_dir), shell=True)

        test_name = f"{fibonacci_bigint_test_name}_{factorized_number}"
        call(self.rename_cmd.format(factorization_test_name, test_name), shell=True)
        return join(out_dir, self.generated_tests_dir, test_name) + ".wasm"

    def __compile_hash_test(self, iterations_count, initial_value, test_full_path, out_dir):
        hash_compile_string = "ITERATIONS_COUNT={} INITIAL_VALUE={} cargo build --manifest-path {}/Cargo.toml " \
                                "--target-dir {} --release --target wasm32-unknown-unknown"
        call(hash_compile_string.format(iterations_count, initial_value, test_full_path, out_dir), shell=True)

        test_name = f"{recursive_hash_test_name}_{iterations_count}_{initial_value}"
        call(self.rename_cmd.format(recursive_hash_test_name, test_name), shell=True)
        return join(out_dir, self.generated_tests_dir, test_name) + ".wasm"

    def __compile_matrix_test(self, seed, matrix_size, iterations_count, test_full_path, out_dir, matrix_test_name):
        matrix_compile_string = "SEED={} MATRIX_SIZE={} ITERATIONS_COUNT={} cargo build --manifest-path {}/Cargo.toml " \
                                "--target-dir {} --release --target wasm32-unknown-unknown"
        call(matrix_compile_string.format(seed, matrix_size, iterations_count, test_full_path, out_dir), shell=True)

        test_name = f"{matrix_test_name}_{seed}_{matrix_size}_{iterations_count}"
        call(self.rename_cmd.format(matrix_test_name, test_name), shell=True)
        return join(out_dir, self.generated_tests_dir, test_name) + ".wasm"
