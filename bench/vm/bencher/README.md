## Webassembly bencher

This bencher developed for benchmarking both Webassembly (Wasm) compilers and interpreters. It has flexible settings provides a capability to run a bunch of tests with predefined parameters (for more info please see `../tests`) on different Wasm process virtual machine. To conduct your own tests, please note that `project/Settings.py` holds settings for process virtual machines and test_settings folder contains `csv` files that adjust tests.

## Quick start

To run bencher you have to specify some arguments: 
- --vm_dir - a directory with Webassembly virtual machines installed
- --tests_dir - a directory with benchmark tests
- --test_settings_file - a full path to file with tests settings
- --out_dir - a directory where results will be saved

```bash
python3 main.py --vm_dir /home/user/vm --tests_dir /home/user/fluence/bench/vm/tests --test_settings_file /home/user/fluence/bench/vm/bencher/test_settings/jit_vs_interpreters.csv --out_dir /home/user/results
```



## Requirements

You need Python 3 and [click] (https://click.palletsprojects.com/en/7.x/) to run bencher. In Ubuntu, Mint and Debian you can install Python 3 and click like this:

```bash
sudo apt-get install python3 python3-pip

pip install click
```

Also note that you have to install Wasm virtual machines at first and also satisfy requirements described in `../test/Readme.md` to be able to launch tests.
