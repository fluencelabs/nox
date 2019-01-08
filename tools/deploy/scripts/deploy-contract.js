let fs = require("fs");
let Web3 = require('web3');

let web3 = new Web3();
web3.setProvider(new web3.providers.HttpProvider('http://localhost:8545'));

let source = fs.readFileSync("../../../bootstrap/contracts/compiled/Network.abi");
let abi = JSON.parse(source);

let sourceBin = fs.readFileSync("../../../bootstrap/contracts/compiled/Network.bin");
// Smart contract EVM bytecode as hex
let code = '0x' + sourceBin;

// Create Contract proxy class
let NetworkContract = new web3.eth.Contract(abi);

let acc = '0x00a329c0648769a73afac7f9381e08fb43dbea72'

// Unlock the coinbase account to make transactions out of it
var password = "";

web3.eth.personal.unlockAccount(acc, password)


let deployer = NetworkContract.deploy({
    data: code
}).encodeABI();

let resp = web3.eth.sendTransaction({ from: acc, data: deployer, gas: '4500000' }, "");

resp.on('error', console.error)
    .then(function(newContractInstance){
        console.log(newContractInstance.contractAddress) // instance with the new contract address
});