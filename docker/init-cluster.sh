rm -rf $1
tendermint testnet --o $1 --hostname-prefix $1_node
echo $(cat $1/node0/config/config.toml | grep persistent_peers | awk '{print $3}') > $1/node0/config/persistent_peers.txt
echo $(cat $1/node1/config/config.toml | grep persistent_peers | awk '{print $3}') > $1/node1/config/persistent_peers.txt
echo $(cat $1/node2/config/config.toml | grep persistent_peers | awk '{print $3}') > $1/node2/config/persistent_peers.txt
echo $(cat $1/node3/config/config.toml | grep persistent_peers | awk '{print $3}') > $1/node3/config/persistent_peers.txt
