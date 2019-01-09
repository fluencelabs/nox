let fs = require("fs");
let Web3 = require('web3');

let web3 = new Web3();
web3.setProvider(new web3.providers.HttpProvider('http://localhost:8545'));

let source = fs.readFileSync("../../../bootstrap/contracts/compiled/Network.abi");
let abi = JSON.parse(source);

let sourceBin = fs.readFileSync("../../../bootstrap/contracts/compiled/Network.bin");
let bytecode = '0x' + sourceBin;

// Create Contract proxy class
let NetworkContract = new web3.eth.Contract(abi);

// Default account on parity node in dev mode with a huge amount of ethereum
let acc = '0x00a329c0648769a73afac7f9381e08fb43dbea72'

// Unlock the coinbase account to make transactions out of it
var password = "";
web3.eth.personal.unlockAccount(acc, password)


let encodedAbi = NetworkContract.deploy({
    data: bytecode
}).encodeABI();

web3.eth.sendTransaction({ from: acc, data: encodedAbi, gas: '4500000' }, "")
.on('error', console.error)
.then(function(newContractInstance){
    console.log(newContractInstance.contractAddress) // instance with the new contract address
});