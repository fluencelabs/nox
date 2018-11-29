#!/usr/bin/python

from BenchTestGenerator import BenchTestGenerator
from WasmVMBencher import WasmVMBencher
from settings import vm_descriptors, test_descriptors
import click
import csv


def save_test_results(results):
    for vm in results:
        with open(vm + ".csv", 'w', newline='') as vm_file:
            fieldnames = ['test_path', 'elapsed_time']
            writer = csv.DictWriter(vm_file, fieldnames=fieldnames)
            writer.writeheader()

            for test_path, result_descriptor in results[vm].items():
                writer.writerow({"test_path" : test_path, "elapsed_time" : result_descriptor.time})


@click.command()
@click.option("--vm_dir", help="directory with Webassembly virtual machines")
@click.option("--tests_dir", help="directory with benchmark tests")
@click.option("--out_dir", help="directory where results will be saved")
def main(vm_dir, tests_dir, out_dir):
    print("<wasm_bencher>: starting generation tests")
    test_generator = BenchTestGenerator(tests_dir)
    filled_tests_descriptors = test_generator.generate_tests(out_dir, test_descriptors)

    print("<wasm_bencher>: starting vm tests")
    vm_bencher = WasmVMBencher(vm_dir)
    test_results = vm_bencher.run_tests(filled_tests_descriptors, vm_descriptors)

    print("<wasm_bencher>: starting collection of test results")
    save_test_results(test_results)


if __name__ == '__main__':
    main()
