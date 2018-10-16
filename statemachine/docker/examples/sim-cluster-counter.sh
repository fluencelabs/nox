#!/bin/bash -e
pushd . > /dev/null
cd ..
./sim-cluster.sh counternet "$PWD/examples/vmcode-counter" 25057 $HOME/.fluence/long-term-keys
popd > /dev/null
echo "Connecting to counternet_node3 logs. Ctrl+C to detach."
echo "Use 'docker logs -f counternet_node3' to reattach"
docker logs -f counternet_node3
