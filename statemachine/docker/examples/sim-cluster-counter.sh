#!/bin/bash -e
pushd . > /dev/null
cd ..
./sim-cluster.sh counternet "$PWD/examples/vmcode-counter" 24057 $HOME/.fluence/long-term-keys
popd > /dev/null
echo "Connecting to counternet_node0 logs. Ctrl+C to detach"
echo "Use 'docker logs -f counternet_node0' to reattach"
docker logs -f counternet_node0
