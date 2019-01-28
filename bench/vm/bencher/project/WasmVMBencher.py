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
from project.settings import interpreter_launches_count,\
    compiler_launches_count, \
    test_export_function_name

from os import listdir
from os.path import join
from time import time
from subprocess import Popen
from collections import defaultdict
import logging


class Record:
    """Contains measures of one test launch.

    Attributes
    ----------
    time : time_type
        The execution time of one test.
    cpu_load : int
        The cpu load (in percents) of one test (currently not supported).

    """
    def __init__(self, time=0.0, cpu_load=0):
        self.time = time
        self.cpu_load = cpu_load  # TODO


class WasmVMBencher:
    """Launches each VM on given directory on each provided test."""

    def __init__(self, vm_dir):
        self.vm_dir = vm_dir
        self.enabled_vm = listdir(vm_dir)

    def run_tests(self, test_descriptors, vm_descriptors):
        """Launches provided tests and returns their execution time.

        Parameters
        ----------
        test_descriptors
            Descriptors of test that should be used for benchmark Wasm VM.
        vm_descriptors
            Descriptors of Wasm VM that should be tested on provided tests.

        Returns
        -------
        {vm : {test_name : [Records]}}
            Collected test results.

       """
        # {vm : {test_name : [Records]}}
        results = defaultdict(lambda: defaultdict(list))
        logger = logging.getLogger("wasm_bencher_logger")

        for test_name, test_descriptor in test_descriptors.items():
            logger.info("<wasm_bencher>: launch {} test".format(test_name))
            for vm in self.enabled_vm:
                if vm not in vm_descriptors:
                    continue

                vm_binary_full_path = join(self.vm_dir, vm, vm_descriptors[vm].vm_relative_binary_path)
                cmd = vm_binary_full_path + " " \
                      + vm_descriptors[vm].vm_launch_cmd.format(wasm_file_path=test_descriptor.generated_test_full_path,
                                                                function_name=test_export_function_name)

                launch_count = compiler_launches_count if vm_descriptors[vm].is_compiler_type \
                    else interpreter_launches_count
                for _ in range(launch_count):
                    logger.info("<wasm_bencher>: {}".format(cmd))
                    result_record = self.__do_one_test(cmd)
                    results[vm][test_name].append(result_record)
                    logger.info("<wasm_bencher>: {} result collected: time={}".format(vm, result_record.time))

        return results

    def __do_one_test(self, vm_cmd):
        """Launches provided shell command string via subprocess.Popen and measure its execution time.

        Parameters
        ----------
        vm_cmd : str
            An exactly command that should be executed.

        Returns
        -------
        time_type
            An elapsed time of provided cmd execution.

        """
        start_time = time()
        Popen(vm_cmd, shell=True).wait(None)
        end_time = time()
        return Record(end_time - start_time)
