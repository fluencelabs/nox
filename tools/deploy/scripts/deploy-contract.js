let fs = require("fs");
let Web3 = require('web3');
let request = require('request');
let Accounts = require('web3-eth-accounts');
let Tx = require('ethereumjs-tx');

// Connect to the local ethereum node
let web3 = new Web3(new Web3.providers.HttpProvider('http://geth.fluence.one:8545'));

// Default account on parity node in dev mode with a huge amount of ethereum
let acc = '0x7DdAE2d6118562AaC405284bb297C9A53d975326';
let privateKey = '3639CC2D3D27ABF76509077EFC0BE6093290F0F8739C00BDDA6504B9D9FC66C2';

if (!acc.startsWith("0x")) {
    acc = "0x" + acc;
}

let privateKeyBuf = Buffer.from(privateKey.replace(/^0x/, ""), 'hex');

let compiledPath = "../../../bootstrap/contracts/compiled/";
function path(file) { return compiledPath + file }
function pause(duration) { return new Promise(resolve => setTimeout(resolve, duration)) }
function print(str) { process.stdout.write(str) }

async function getReceipt(hash, tries = 10, wait = 2000, count = 0) {
    let receipt = await web3.eth.getTransactionReceipt(hash);
    if (receipt === null || receipt === undefined) {
        if (count < tries) {
            print(".");
            await pause(wait);
            return await getReceipt(hash, tries, wait, count + 1);
        } else {
            await Promise.reject(`Unable to retrieve tx receipt after ${count} tries.`);
        }
    } else {
        return receipt;
    }
}

async function deployContract(name, deployArgument) {
    print(`Deploying contract ${name}`);
    // Get ABI from compiled contract
    let abiPath = path(name + ".abi");
    let abiJSON = fs.readFileSync(abiPath);
    let abi = JSON.parse(abiJSON);

    // Get bytecode from compiled contract
    let sourcePath = path(name + ".bin");
    let sourceBin = fs.readFileSync(sourcePath);
    let bytecode = '0x' + sourceBin;
    print(".");

    // Create Contract proxy class
    let contract = new web3.eth.Contract(abi);

    // Get encoded request for deploying contract
    let encodedAbi = contract.deploy({
        data: bytecode,
        arguments: deployArgument
    }).encodeABI();

    let nonceHex = await web3.eth.getTransactionCount(acc, "pending").then(nonce => web3.utils.numberToHex(nonce));
    let gasPriceHex = await web3.eth.getGasPrice().then(gasPrice => web3.utils.numberToHex(gasPrice));
    let gasLimitHex = await web3.utils.numberToHex(4500000);

    let tx = new Tx({
        nonce: nonceHex,
        gasPrice: gasPriceHex,
        gasLimit: gasLimitHex,
        data: encodedAbi,
        from: acc
    });
    tx.sign(privateKeyBuf);
    let signedTx = tx.serialize();

    let rawTx = '0x' + signedTx.toString('hex');
    print(".");

    // Using `new Promise` here since just `await sendSignedTransaction` didn't work - it hanged forever, don't know why
    let receipt = await new Promise((resolve, reject) => {
        web3.eth.sendSignedTransaction(rawTx, async (error, hash) => {
            getReceipt(hash).then(receipt => {
                print("OK\n");
                console.log(`tx hash: ${hash}`);
                resolve(receipt)
            }).catch(e => {
                print("FAIL\n");
                reject(`tx hash: ${hash}\n${e}`)
            });
        });
    });

    return receipt.contractAddress;
}

async function deployNetwork() {
    let network = await deployContract("Network").catch(e => console.error("Error deploying Network: " + e));
    console.log(`Network contract: ${network}`);
    return network;
}

async function deployDashboard(network) {
    let dashboard = await deployContract("Dashboard", [network]).catch(e => console.error("Error deploying Dashboard: " + e));
    console.log(`Dashboard contract: ${dashboard}`);
    return dashboard;
}

async function deployBoth() {
    let network = await deployNetwork();
    console.log("\n");
    await deployDashboard(network);
}

if (process.argv.length >= 3) {
    if (process.argv[2] === "network") {
        return deployNetwork();
    } else if (process.argv[2] === "dashboard") {
        let network = process.argv[3];
        if (network === null || network === undefined) {
            console.error("Network address should be specified as a second argument");
        } else {
            console.log(`Network address is ${network}`);
            return deployDashboard(network);
        }
    } else {
        console.error(`Unknown deployment type ${process.argv[2]}`)
    }
} else {
    return deployBoth();
}
