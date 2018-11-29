from os.path import join
from settings import *
from subprocess import call


class BenchTestGenerator:

    def __init__(self, test_dir):
        self.tests_dir = test_dir
        self.generated_tests_dir = "bench_tests"
        self.rename_cmd = ""

    # compiles test with chosen parameters
    def generate_tests(self, out_dir, test_descriptors):
        call("mkdir {}".format(join(out_dir,self.generated_tests_dir)),shell=True)

        # to clean possible previous unsuccessfully build results that prevent cargo to build new tests
        call(("rm -rf " + join("{}", "wasm32-unknown-unknown")).format(out_dir), shell=True)
        call(("rm -rf " + join("{}", "release")).format(out_dir), shell=True)

        generated_tests_dir_full_path = join(out_dir, self.generated_tests_dir)
        test_mv_cmd = "mv " + join(out_dir, "wasm32-unknown-unknown", "release", "{}.wasm") + " " \
                          + join(generated_tests_dir_full_path, "{}.wasm")

        for test_name, test_descriptor in test_descriptors.items():
            test_full_path = join(self.tests_dir, test_descriptor.test_folder_name)
            test_cmd = test_descriptor.test_generator_cmd.format(test_full_path, out_dir)

            for key, value in test_descriptor.test_generator_params.items():
                test_cmd = "{}={} {}".format(key, value, test_cmd)
            call(test_cmd, shell=True)
            call(test_mv_cmd.format(test_descriptor.test_folder_name, test_name), shell=True)

            # collect garbage to force cargo build the same test with different env params again
            call(("rm -rf " + join("{}", "wasm32-unknown-unknown")).format(out_dir), shell=True)
            call(("rm -rf " + join("{}", "release")).format(out_dir), shell=True)

            test_descriptors[test_name].test_full_path = join(generated_tests_dir_full_path, "{}.wasm").format(test_name)

        return test_descriptors
