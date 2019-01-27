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


class VMDescriptor:
    """Wasm VM descriptor that specifies how VM has to be launched.

    Attributes
    ----------
    vm_relative_binary_path : str
        A relative path to VM binary in its main folder.
    vm_launch_cmd : str
        An format string with command for launch this vm with provided test.
    is_compiler_type : bool
        True, if vm is compiler-type (JIT, AOT, ...).

    """
    def __init__(self, vm_relative_binary_path="", vm_launch_cmd="", is_compiler_type=True):
        self.vm_relative_binary_path = vm_relative_binary_path
        self.vm_launch_cmd = vm_launch_cmd
        self.is_compiler_type = is_compiler_type
