#!/usr/bin/python

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
from project.BenchTestGenerator import BenchTestGenerator
from project.WasmVMBencher import WasmVMBencher
from project.settings import vm_descriptors
from project.TestDescriptor import TestDescriptor

import click
import csv
import logging
from os.path import join


def save_test_results(out_dir, results):
    """Saves provided results to <vm_name>.csv files in a given directory.

    Parameters
    ----------
    out_dir : str
        A directory where the result will be saved.
    results : {vm_name : {test_name : [Record]}}
        Results that should be saved.

    """
    for vm in results:
        with open(join(out_dir, vm + ".csv"), 'w', newline='') as bench_result_file:
            fieldnames = ['test_name', 'elapsed_time']
            writer = csv.DictWriter(bench_result_file, fieldnames=fieldnames)
            writer.writeheader()

            for test_name, result_records in results[vm].items():
                for record in result_records:
                    writer.writerow({"test_name" : test_name, "elapsed_time" : record.time})


def load_test_descriptors(test_settings_dir):
    """Loads tests settings from a given csv file.

    Parameters
    ----------
    test_settings_dir : str
        A full path to file with settings.

    Returns
    -------
    results : {test_name : TestDescriptor}
        Resulted dictionary with test descriptors.

    """
    results = {}
    with open(test_settings_dir, 'r', newline='') as test_settings_file:
        reader = csv.DictReader(test_settings_file)
        for rows in reader:
            results[rows["test_name"]] = TestDescriptor(
                rows["test_folder_name"],
                rows["test_compilation_cmd"],
                rows["test_compilation_parameters"]
            )

    return results


@click.command()
@click.option("--vm_dir", help="a directory with Webassembly virtual machines")
@click.option("--tests_dir", help="a directory with benchmark tests")
@click.option("--test_settings_dir", help="a full path to file with tests settings")
@click.option("--out_dir", help="a directory where results will be saved")
def main(vm_dir, tests_dir, test_settings_dir, out_dir):
    logging.basicConfig(filename="wasm_bencher_log", level=logging.INFO, format='%(asctime)s %(message)s',
                        datefmt='%m/%d/%Y %I:%M:%S %p')

    logger = logging.getLogger("wasm_bench_logger")

    logger.info("<wasm_bencher>: starting tests generation")
    test_descriptors = load_test_descriptors(test_settings_dir)
    test_generator = BenchTestGenerator(tests_dir)
    filled_tests_descriptors = test_generator.generate_tests(out_dir, test_descriptors)

    logger.info("<wasm_bencher>: starting vm tests")
    vm_bencher = WasmVMBencher(vm_dir)
    test_results = vm_bencher.run_tests(filled_tests_descriptors, vm_descriptors)

    logger.info("<wasm_bencher>: starting collection of test results")
    save_test_results(out_dir, test_results)


if __name__ == '__main__':
    main()
