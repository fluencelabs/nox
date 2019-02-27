let fs = require("fs");
let Web3 = require('web3');
let request = require('request');
let Accounts = require('web3-eth-accounts');
let Tx = require('ethereumjs-tx');


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
let privateKey = '0x4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7';

if (!acc.startsWith("0x")) {
    acc = "0x" + acc;
}

let privateKeyBuf = Buffer.from(privateKey.replace(/^0x/, ""), 'hex');

// Get encoded request for deploying contract
let encodedAbi = NetworkContract.deploy({
    data: bytecode
}).encodeABI();

let nonceHexP = web3.eth.getTransactionCount(acc, "pending").then(nonce => web3.utils.numberToHex(nonce));
let gasPriceHexP = web3.eth.getGasPrice().then(gasPrice => web3.utils.numberToHex(gasPrice));
let gasLimitHex = web3.utils.numberToHex(4500000);

let rawTxP = nonceHexP.then(nonceHex => gasPriceHexP.then(gasPriceHex => {
    return {
        nonce: nonceHex,
        gasPrice: gasPriceHex,
        gasLimit: gasLimitHex,
        data: encodedAbi,
        from: acc
    }
}));

let signedTxP = rawTxP.then(rawTx => {
    let tx = new Tx(rawTx);
    tx.sign(privateKeyBuf);

    return tx.serialize()
}).catch(e => {
    console.error("tx signing failed: " + e);
    process.exit(1);
});

signedTxP.then(rawTx => {
    web3.eth.sendSignedTransaction('0x' + rawTx.toString('hex'), (err, hash) => {
        if (err) {
            console.log(err);
            return;
        }

        // Log the tx, you can explore status manually with eth.getTransaction()
        console.log('Contract creation tx: ' + hash);
        web3.eth.getTransactionReceipt(hash).then((receipt) => {
            // will be null on light node, use etherescan for non-dev chains
            if (receipt !== null && receipt !== undefined && receipt.contractAddress !== undefined && receipt.contractAddress !== null) {
                console.log(receipt.contractAddress);
            } else {
                console.log("CONTRACT ADDRESS IS NULL")
            }
        })
    })
}).catch(e => {
    console.error("Error while sending signed transaction");
    process.exit(1);
});

