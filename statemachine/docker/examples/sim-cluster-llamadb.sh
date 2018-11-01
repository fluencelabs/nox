#!/bin/bash -e
mkdir -p vmcode-llamadb
cp ../../../vm/src/test/resources/wasm/llamadb.wasm vmcode-llamadb
pushd . > /dev/null
cd ..
./sim-cluster.sh llamadbnet "$PWD/examples/vmcode-llamadb" 25057 $HOME/.fluence/long-term-keys
popd > /dev/null
echo "Connecting to llamadbnet_node0 logs. Ctrl+C to detach"
echo "Use 'docker logs -f llamadbnet_node0' to reattach"
docker logs -f llamadbnet_node0
