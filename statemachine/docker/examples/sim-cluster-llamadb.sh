#!/bin/bash -e
pushd . > /dev/null
cd ..
./sim-cluster.sh llamadbnet "$PWD/examples/vmcode-llamadb" 29057 $HOME/.fluence/long-term-keys
popd > /dev/null
echo "Connecting to llamadbnet_node3 logs. Ctrl+C to detach."
echo "Use 'docker logs -f llamadbnet_node3' to reattach"
docker logs -f llamadbnet_node3
