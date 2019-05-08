var fs = require('fs');

var solc = require('solc');
var path = require('path');

var zeppelinPath = __dirname + '/../node_modules/openzeppelin-solidity/contracts';

var compiledPath = __dirname + '/../contracts/compiled'

var input = {
    language: 'Solidity',
    sources: {
        'Deployer.sol': {
            content: fs.readFileSync(__dirname + '/../contracts/Deployer.sol', 'utf8')
        },
        'Network.sol': {
            content: fs.readFileSync(__dirname + '/../contracts/Network.sol', 'utf8')
        },
        'Dashboard.sol': {
            content: fs.readFileSync(__dirname + '/../contracts/Dashboard.sol', 'utf8')
        }
    },
    settings: {
        outputSelection: {
            '*': {
                '*': [ '*' ]
            }
        }
    }
};

var output = JSON.parse(solc.compile(JSON.stringify(input)));

if (!output) {
    abort('No output from compiler');
} else if (output.errors) {
    let hasErrors = false;
    for (let error in output.errors) {
        let message = output.errors[error];
        console.log(message.formattedMessage);
        if (message.severity.includes("error")) {
            hasErrors = true;
        }
    }
    if (hasErrors) {
        console.error("Aborting compilation due to errors.");
        abort('Errors occured.');
    }
}

function writeFile(file, content) {
    fs.writeFile(file, content, function (err) {
        if (err) {
          console.error('Failed to write ' + file + ': ' + err);
        }
    });
}

if (!fs.existsSync(compiledPath)){
    fs.mkdirSync(compiledPath);
}

function writeContract(name) {
    let contract = output.contracts[`${name}.sol`][`${name}`];
    if (contract.evm.bytecode === undefined) {
        console.error(`${name}.evm.bytecode is undefined after compilation`);
        abort(`${name}.evm.bytecode is undefined after compilation`)
    }
    if (contract.abi === undefined) {
        console.error(`${name}.abi is undefined after compilation`);
        abort(`${name}.abi is undefined after compilation`)
    }

    writeFile(compiledPath + `/${name}.bin`, contract.evm.bytecode.object);
    writeFile(compiledPath + `/${name}.abi`, JSON.stringify(contract.abi));
}

writeContract("Network");
writeContract("Dashboard");
