## Running private blockchain for Deployer contract

### Launching new private blockchain 

Run these commands from `tools/etc` directory

```
// initialize node1
geth --datadir "$(pwd)/node1" init genesis.json

// copy predefined keystore file to node1 keystore
cp UTC--2018-10-26T04-39-22.430742000Z--24b2285cfc8a68d1beec4f4282ee6016aebb8fc4 node1/keystore
// it would initialize 0x24b2285cfc8a68d1beec4f4282ee6016aebb8fc4 account with enough eth

// run node1
geth --datadir "$(pwd)/node1" --networkid 1234 --port 11111 --nodiscover console
```

### Obtaining Deployer.abi and Deployer.bin

From `bootstrap` run `play.sh` and copy `deployerAbi` and `deployerHex` definitions (all output starting from `deployerAbi = `).

Paste this output to `geth` console (see previous paragraph).

### Deploying Deployer contract

After pasting `deployerAbi` and `deployerHex` in the console, run the following in the console:

```
// unlock 0x24b2285cfc8a68d1beec4f4282ee6016aebb8fc4 account with 123 password
personal.unlockAccount(eth.accounts[0], "123", 0)

// deploy and mine
deployerInterface = eth.contract(deployerAbi)
deployerTx = deployerInterface.new( { from: eth.accounts[0], data: deployerHex, gas: 100000000 } )
deployerTxHash = deployerTx.transactionHash
miner.start()
admin.sleep(4)
miner.stop()

// take address
publishedDeployerAddr = eth.getTransactionReceipt(deployerTxHash).contractAddress
// take contract
deployerContract = deployerInterface.at(publishedDeployerAddr)

// define event listeners
string2Bytes32 = function(str) { return web3.padRight(web3.toHex(str), 64 + 2) }
newSolverEvent = deployerContract.NewSolver()
newSolverEvent.watch(function (error, result) { if (!error) console.log("NEW_SOLVER " + ": " + JSON.stringify(result)); else console.log("ERROR: " + error); });
clusterFormedEvent = deployerContract.ClusterFormed()
clusterFormedEvent.watch(function (error, result) { if (!error) console.log("CLUSTER_FORMED " + ": " + JSON.stringify(result)); else console.log("ERROR: " + error); });

// check status: it should return somethink like: '101, 0, []'
// if the first number is 0, the contract not deployed correctly :( 
deployerContract.getStatus({ from: eth.accounts[0] })

// start RPC on 8545 port
admin.startRPC("localhost", 8545)
```

### Adding code and solvers

This would add code for 4-node llamadb cluster and start mining: 

```
deployerContract.addCode(string2Bytes32("llamadb"), string2Bytes32("receipt_placeholder"), 1, new Date().getTime() + new Date().getTimezoneOffset() * 60000, { from: eth.accounts[0], gas: 1000000 })
miner.start()

```

Test adding solvers immediately in console (this would not launch any solvers, of course):

```
deployerContract.addSolver(string2Bytes32("solver0"), string2Bytes32("addrsome"), { from: eth.accounts[0], gas: 1000000 })
deployerContract.addSolver(string2Bytes32("solver1"), string2Bytes32("addrsome"), { from: eth.accounts[0], gas: 1000000 })
deployerContract.addSolver(string2Bytes32("solver2"), string2Bytes32("addrsome"), { from: eth.accounts[0], gas: 1000000 })
deployerContract.addSolver(string2Bytes32("solver3"), string2Bytes32("addrsome"), { from: eth.accounts[0], gas: 1000000 })
```

Alternatively, `MasterNodeApp` from `ethclient` might be launched for each solver.

Stop mining as soon as needed:

```
miner.stop()
```
