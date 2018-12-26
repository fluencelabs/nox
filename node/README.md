## How to run ganache
In order to run master node, you need to have any Ethereum RPC node. In this example we will use Ganache for that purpose. To run Ganache and deploy Fluence contract there, run
```bash
cd fluence/bootstrap

npm install
npm run ganache
npm run migrate
```

You will see address of the Fluence smart contract in your private Ganache blockchain:
```
0x9995882876ae612bfd829498ccd73dd962ec950a
```

Ganache is run in deterministic mode with predefined mnemonic, so this address will always be the same and there is no need to save it.

## How to run master node
Go to `fluence/tools/node` and run `docker-compose`:
```bash
cd fluence/tools/node

docker-compose -f single-master.yml up -dV
```

`single-master.yml` runs one Swarm container and one master node container with solver ports 25000-25003.

## How to run solvers

After having run master node, you need to upload and publish your WASM code to Swarm. In this example we will use `llamadb` that can be built by 
```bash
sbt vm-llamadb/compile
```
from the project root directory. Then you can use Fluence `cli` to upload and publish it:

```bash
cd fluence/cli

cargo build --release

target/release/fluence publish ../vm/examples/llamadb/target/wasm32-unknown-unknown/release/llama_db.wasm 0x9995882876ae612bfd829498ccd73dd962ec950a 0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d --cluster_size 1
```

You can take a look at `target/release/fluence publish --help` to get the idea of how it works. I will describe parameters briefly:

- `target/release/fluence` is a `cli` binary we compiled by running `cargo build --release`
- `publish` is a command that uploads code to Swarm and sends it to Ethereum contract (running on Ganache)
- `../vm/examples/llamadb/target/wasm32-unknown-unknown/release/llama_db.wasm` is a path to llamadb compiled to WASM
- `0x9995882876ae612bfd829498ccd73dd962ec950a` is a Fluence contract address that was deployed by `npm run migrate`
- `0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d` is an Ethereum account address that's used for publishing code
- `--cluster_size 1` specifies number of nodes required to run this code. We ran single master node, so we specify 1 here.

## How to check solver is running fine
You can check solver's logs by issuing `docker logs 01_node0` (you may need to replace `01_node0` with the name of the solver container, see `docker ps`). Look for `Commit: processTime=23 height=2`, `processTime` could be anything, you're looking for `height=2`. It can appear after some time, usually 10-60 seconds, depending on your hardware.

## Cleaning up
- To clean up ALL docker containers, including volumes: `docker ps -a | awk '{print $1}' | xargs docker rm -f ; docker volume prune -f`
- To clean up docker-compose state: `docker-compose -f single-master.yml kill`
- To stop ganache: `pkill -f ganache`

## Troubleshooting
TODO
