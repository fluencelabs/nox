var fs = require('fs');

var solc = require('solc');
var path = require('path');

var zeppelinPath = __dirname + '/../node_modules/openzeppelin-solidity/contracts';

var compiledPath = __dirname + '/../contracts/compiled'

var input = {
    'Deployer.sol': fs.readFileSync(__dirname + '/../contracts/Deployer.sol', 'utf8'),
    'Network.sol': fs.readFileSync(__dirname + '/../contracts/Network.sol', 'utf8')
};

var output = solc.compile({ sources: input }, 1)

if (!output) {
    abort('No output from compiler');
} else if (output.errors) {
    let hasErrors = false;
    for (var error in output.errors) {
        var message = output.errors[error]
	if (!message.includes("Warning:")) {
		hasErrors = true;
	}
    }
    if (hasErrors) abort('Errors occured.');
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

let networkContract = output.contracts["Network.sol:Network"];
writeFile(compiledPath + '/Network.bin', networkContract.bytecode);
writeFile(compiledPath + '/Network.abi', networkContract.interface);
