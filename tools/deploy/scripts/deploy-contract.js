let fs = require("fs");
let Web3 = require('web3');
let request = require('request');
let Accounts = require('web3-eth-accounts');

// Connect to the local ethereum node
let web3 = new Web3();
web3.setProvider(new web3.providers.HttpProvider('http://localhost:8545'));

// Get ABI from compiled contract
let source = fs.readFileSync("../../../bootstrap/contracts/compiled/Network.abi");
let abi = JSON.parse(source);

// Get bytecode from compiled contract
let sourceBin = fs.readFileSync("../../../bootstrap/contracts/compiled/Network.bin");
let bytecode = '0x' + sourceBin;

// Create Contract proxy class
let NetworkContract = new web3.eth.Contract(abi);

// Default account on parity node in dev mode with a huge amount of ethereum
let acc = '0x00a329c0648769a73afac7f9381e08fb43dbea72';

// Get encoded request for deploying contract
let encodedAbi = NetworkContract.deploy({
    data: bytecode
}).encodeABI();

// create partial transaction
let partialTransaction = { from: acc, data: encodedAbi, gas: web3.utils.numberToHex('4500000') };

// request to compose partial transaction to full transaction
let composeRequest = {method:"parity_composeTransaction",params:[partialTransaction],"id":1,"jsonrpc":"2.0"};

request.post(
    'http://localhost:8545/',
    { json: composeRequest },
    function (error, response, body) {
        // sign transaction with private key
        web3.eth.accounts.signTransaction(body.result, '0x4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7')
            .then((resp) => {
                // send sign transaction to the node
                web3.eth.sendSignedTransaction(resp.rawTransaction, function (err, transactionHash) {
                    web3.eth.getTransactionReceipt(transactionHash).then((receipt) => {
                    // will be null on light node, use etherescan for non-dev chains
                    console.log(receipt.contractAddress);
                    })

                });
            })
    }
);
