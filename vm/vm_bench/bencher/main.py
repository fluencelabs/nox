#!/usr/bin/python

from BenchTestGenerator import BenchTestGenerator
from WasmVMBencher import WasmVMBencher
from settings import vm_descriptors
import click
import csv


def save_test_results(results):
    for vm in results:
        with open(vm + ".csv", 'w', newline='') as vm_file:
            fieldnames = ['test_path', 'elapsed_time']
            writer = csv.DictWriter(vm_file, fieldnames=fieldnames)
            writer.writeheader()

            for test_path, result_descriptor in results[vm].iteritems():
                writer.writerow({"test_path" : test_path, "elapsed_time" : result_descriptor.time})


@click.command()
@click.option("--vm_dir", help="directory with Webassembly virtual machines")
@click.option("--tests_dir", help="directory with benchmark tests")
@click.option("--out_dir", help="directory where results will be saved")
def main(vm_dir, tests_dir, out_dir):
    print("<wasm_bencher>: starting tests generation")
    test_generator = BenchTestGenerator(tests_dir)
    tests_path = test_generator.generate_tests(out_dir)

    print("<wasm_bencher>: starting vm launching")
    vm_bencher = WasmVMBencher(vm_dir)
    test_results = vm_bencher.run_tests(tests_path, vm_descriptors)

    print("<wasm_bencher>: starting tests result collection")
    save_test_results(test_results)


if __name__ == '__main__':
    main()
