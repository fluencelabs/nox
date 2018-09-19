# Setting up private Ethereum blockchain with a full and a light nodes

## Prerequisites
You will need [Docker](https://www.docker.com/get-started) installed and running.

## Create directory for blockchain
`BLOCKCHAIN=/some/path`
`mkdir $BLOCKCHAIN`

## Create data dir
You need to run this for every future node, specifying different datadir since datadir can't be shared.
`docker run -v "$BLOCKCHAIN:/root" ethereum/client-go --networkid=1234 --datadir="/root/.ethereum/devchain" --mine init "/root/genesis.json"`

## Create docker network so nodes could freely talk to each other
`docker create network geth-net`

## Run first node, full one
`docker run -d -p 8545:8545 -p 30303:30303 --name private-geth1 -v "$BLOCKCHAIN:/root" --network=geth-net ethereum/client-go --nodiscover --rpcapi "db,personal,eth,net,web3" --rpccorsdomain '*' --networkid 1234 --rpc --rpcaddr "0.0.0.0" --datadir "/root/.ethereum/devchain" --lightserv 25`

Note, `--lightserv 25` is very important here, since it says how much of CPU % node willing to allocate on light clients. Default is ZERO, so lightclient (second node) wouldn't be able to connect

## Check logs through
`docker logs private-geth1`
Also, be aware of `--tail 1000` and `--follow` options, they are useful.

## Something like that will appear in the logs, that's node address
```
INFO [08-27|14:44:01.217] UDP listener up                          self=enode://da3758291fe30ab2cd9b5f055e9e091130cf205f96a285d4afb21bf95bf14ef07ef2dab8ab8dfc639ac769ee98ad42daf27cef2f3cb161fd35ba9c4fe5959b31@[::]:30303
INFO [08-27|14:44:01.218] RLPx listener up                         self=enode://da3758291fe30ab2cd9b5f055e9e091130cf205f96a285d4afb21bf95bf14ef07ef2dab8ab8dfc639ac769ee98ad42daf27cef2f3cb161fd35ba9c4fe5959b31@192.168.1.71:30303
```

## Run second node, light one
`docker run -d -p 8546:8545 -p 30304:30304 --name private-geth2 -v "$BLOCKCHAIN:/root" --network=geth-net ethereum/client-go --nodiscover --syncmode "light" --rpcapi "db,personal,eth,net,web3" --rpccorsdomain '*' --networkid 1234 --rpc --rpcaddr "0.0.0.0" --datadir "/root/.ethereum/devchain2" --port 30304 --lightserv 25`

## Create account (wallet)
`docker exec -it private-geth2 geth --datadir "/root/.ethereum/console" --networkid 1234 account new`
`cp $BLOCKCHAIN/.ethereum/console/keystore/* $BLOCKCHAIN/.ethereum/devchain/keystore/*`
`cp $BLOCKCHAIN/.ethereum/console/keystore/* $BLOCKCHAIN/.ethereum/devchain2/keystore/*`

## Attach to running geth node
`docker exec -it private-geth2 geth attach --datadir /root/.ethereum/devchain2`

## Add peer
Inside geth console (attach):
`admin.addPeer(enode://abcde1234@LOCAL-IP:30303)`
`LOCAL-IP` should look like `172.18.0.2` or something else, depending or docker network you created. You can check it out via running `ifconfig` inside container.

## Set ether base account to receive mining bounty
Inside geth console
`miner.setEtherbase(eth.accounts[0])`

## Start miner
`miner.start()`

## Make some transactions to check that light client (second node) sees everything
Commands to play with:
- `personal.newAccount`
- `personal.unlockAccount`
- `eth.getBalance`
- `eth.sendTransaction({ from: eth.accounts[0], to: "0xdd04d0743a14c1a58b684e12e43c5874add25c53", value: 200000000000000000000 })`

## Links
https://arvanaghi.com/blog/how-to-set-up-a-private-ethereum-blockchain-using-geth/
https://www.codeooze.com/blockchain/ethereum-geth-private-blockchain/