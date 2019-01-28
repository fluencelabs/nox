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
from subprocess import call


class BenchTestGenerator:
    """Generates tests in provided directory."""

    def __init__(self, test_dir):
        self.tests_dir = test_dir
        self.generated_tests_dir = "bench_tests"

    def generate_tests(self, out_dir, test_descriptors):
        """Generates tests by their descriptors.

        Compiles each test by test_generator_cmd in given test descriptor, moves it to out_dir/generated_tests_dir
        and finally sets test_full_path in each test descriptor.

        Parameters
        ----------
        out_dir : str
            A directory where the resulted test will be saved.
        test_descriptors : [TestDescriptor]
            Descriptors of tests that specifies how exactly test should be compiled.

        Returns
        -------
        [TestDescriptor]
            Test descriptors with test_full_path filled.

        """
        call("mkdir -p {}".format(join(out_dir,self.generated_tests_dir)), shell=True)

        generated_tests_dir_full_path = join(out_dir, self.generated_tests_dir)
        test_mv_cmd = "mv {} {}".format(
            join(out_dir, "wasm32-unknown-unknown", "release", "{}.wasm"),
            join(generated_tests_dir_full_path, "{}.wasm")
        )

        for test_name, test_descriptor in test_descriptors.items():
            test_full_path = join(self.tests_dir, test_descriptor.test_folder_name)
            test_compilation_cmd = "{} {}".format(
                test_descriptor.test_compilation_parameters,
                test_descriptor.test_compilation_cmd.format(test_full_path, out_dir)
            )

            # collect garbage to force cargo build the same test with different env params again
            self.__collect_garbage(out_dir)

            call(test_compilation_cmd, shell=True)
            call(test_mv_cmd.format(test_descriptor.test_folder_name, test_name), shell=True)

            test_descriptors[test_name].generated_test_full_path = \
                join(generated_tests_dir_full_path, "{}.wasm").format(test_name)

        return test_descriptors

    def __collect_garbage(self, out_dir):
        """Removes rust cargo target directories.
        Attributes
        ----------
        out_dir : str
            A directory where build results are placed.

        """
        # Suddenly cargo clean deletes all content of directory given to it as a target-dir
        # so it is necessary to clean garbage "by hands"
        call(("rm -rf {}".format(join("{}", "wasm32-unknown-unknown")).format(out_dir)), shell=True)
        call(("rm -rf {}".format(join("{}", "release")).format(out_dir)), shell=True)
