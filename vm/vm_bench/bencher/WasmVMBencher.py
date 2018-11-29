from settings import interpretator_launch_count, compiler_launch_count, test_function_name

from os import listdir
from os.path import join
from time import time
from subprocess import Popen
from collections import defaultdict


class Record:
    def __init__(self, time=0, cpuLoad=0):
        self.time = time
        self.cpuLoad = cpuLoad  # TODO


class WasmVMBencher:

    def __init__(self, vm_dir):
        self.vm_dir = vm_dir
        self.enabled_vm = listdir(vm_dir)

    def run_tests(self, test_descriptors, vm_descriptors):
        # {{[]}}
        results = defaultdict(lambda: defaultdict(list))

        for test_name, test_descriptor in test_descriptors.items():
            print("<wasm_bencher>: launch test", test_name)
            for vm in self.enabled_vm:
                if vm not in vm_descriptors:
                    pass

                vm_binary_full_path = join(self.vm_dir, vm, vm_descriptors[vm].relative_vm_binary_path)
                cmd = vm_binary_full_path + " " \
                      + vm_descriptors[vm].arg_str.format(wasm_file_path=test_descriptor.test_full_path,
                                                          function_name=test_function_name)

                launch_count = compiler_launch_count if vm_descriptors[vm].is_compiler_type \
                    else interpretator_launch_count
                for _ in range(launch_count):
                    print("<wasm_bencher>: launch cmd", cmd)
                    result_record = self.__do_one_test(cmd)
                    results[vm][test_name].append(result_record)
                    print("<wasm_bencher>: result collected: time={}".format(result_record.time))

        return results

    def __do_one_test(self, vm_cmd):
        start_time = time()
        Popen(vm_cmd, shell=True).wait(None)
        end_time = time()
        return Record(end_time - start_time)
