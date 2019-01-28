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


class TestDescriptor:
    """Descriptor of test that contains parameters for adjusting test compilation process.

    Attributes
    ----------
    test_folder_name : str
        A name of folder where test is located.
    test_compilation_cmd : str
        A compilation string that has to be used to build the test.
    test_compilation_parameters : str
        Parameters that used in test compilation.
    generated_test_full_path : str
        A full path of finally compiled test.

    """
    def __init__(self, test_folder_name="", test_generator_cmd="", test_generator_parameters=""):
        self.test_folder_name = test_folder_name
        self.test_compilation_cmd = test_generator_cmd
        self.test_compilation_parameters = test_generator_parameters
        self.generated_test_full_path = ""
