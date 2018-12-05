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
        """
            Compiles tests by their descriptors.

            Arguments:
                out_dir
                    A directory where the resulted test will be saved.
                test_descriptors
                    Descriptors of tests that specifies how excatly test should be compiled.
        """
        call("mkdir {}".format(join(out_dir,self.generated_tests_dir)), shell=True)

        # collect garbage to clean possible previous unsuccessfully build results that prevent cargo to build new tests
        self.__collect_garbage(out_dir)

        generated_tests_dir_full_path = join(out_dir, self.generated_tests_dir)
        test_mv_cmd = "mv " + join(out_dir, "wasm32-unknown-unknown", "release", "{}.wasm") + " " \
                          + join(generated_tests_dir_full_path, "{}.wasm")

        for test_name, test_descriptor in test_descriptors.items():
            test_full_path = join(self.tests_dir, test_descriptor.test_folder_name)
            test_compile_cmd = test_descriptor.test_generator_cmd.format(test_full_path, out_dir)

            for key, value in test_descriptor.test_generator_params.items():
                test_cmd = "{}={} {}".format(key, value, test_compile_cmd)

            call(test_compile_cmd, shell=True)
            call(test_mv_cmd.format(test_descriptor.test_folder_name, test_name), shell=True)

            # collect garbage to force cargo build the same test with different env params again
            self.__collect_garbage(out_dir)

            test_descriptors[test_name].test_full_path = join(generated_tests_dir_full_path, "{}.wasm").format(test_name)

        return test_descriptors

    def __collect_garbage(self, out_dir):
        call(("rm -rf " + join("{}", "wasm32-unknown-unknown")).format(out_dir), shell=True)
        call(("rm -rf " + join("{}", "release")).format(out_dir), shell=True)
