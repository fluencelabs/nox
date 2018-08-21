cd ../..

screen -d -m -S app1 bash -c 'sbt "statemachine/run 46158"'
screen -d -m -S app2 bash -c 'sbt "statemachine/run 46258"'
screen -d -m -S app3 bash -c 'sbt "statemachine/run 46358"'
screen -d -m -S app4 bash -c 'sbt "statemachine/run 46458"'

cd statemachine/local-cluster-commands

screen -d -m -S tm1 bash -c 'tendermint node --home=$HOME/.tendermint1 --consensus.create_empty_blocks=false --proxy_app=tcp://127.0.0.1:46158 --rpc.laddr=tcp://0.0.0.0:46157 --p2p.laddr=tcp://0.0.0.0:46156 --p2p.persistent_peers=$TM_PERSISTENT_PEERS'
screen -d -m -S tm2 bash -c 'tendermint node --home=$HOME/.tendermint2 --consensus.create_empty_blocks=false --proxy_app=tcp://127.0.0.1:46258 --rpc.laddr=tcp://0.0.0.0:46257 --p2p.laddr=tcp://0.0.0.0:46256 --p2p.persistent_peers=$TM_PERSISTENT_PEERS'
screen -d -m -S tm3 bash -c 'tendermint node --home=$HOME/.tendermint3 --consensus.create_empty_blocks=false --proxy_app=tcp://127.0.0.1:46358 --rpc.laddr=tcp://0.0.0.0:46357 --p2p.laddr=tcp://0.0.0.0:46356 --p2p.persistent_peers=$TM_PERSISTENT_PEERS'
screen -d -m -S tm4 bash -c 'tendermint node --home=$HOME/.tendermint4 --consensus.create_empty_blocks=false --proxy_app=tcp://127.0.0.1:46458 --rpc.laddr=tcp://0.0.0.0:46457 --p2p.laddr=tcp://0.0.0.0:46456 --p2p.persistent_peers=$TM_PERSISTENT_PEERS'
