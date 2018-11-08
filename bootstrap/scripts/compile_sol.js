var fs = require('fs');

var solc = require('solc');
var path = require('path');

var zeppelinPath = __dirname + '/../node_modules/openzeppelin-solidity/contracts';

var compiledPath = __dirname + '/../contracts/compiled'

var input = {
    'Deployer.sol': fs.readFileSync(__dirname + '/../contracts/Deployer.sol', 'utf8'),
    'openzeppelin-solidity/contracts/access/rbac/Roles.sol': fs.readFileSync(zeppelinPath + '/access/rbac/Roles.sol', 'utf8'),
    'openzeppelin-solidity/contracts/ownership/Ownable.sol': fs.readFileSync(zeppelinPath + '/ownership/Ownable.sol', 'utf8'),
    'openzeppelin-solidity/contracts/access/rbac/RBAC.sol': fs.readFileSync(zeppelinPath + '/access/rbac/RBAC.sol', 'utf8'),
    'openzeppelin-solidity/contracts/access/Whitelist.sol': fs.readFileSync(zeppelinPath + '/access/Whitelist.sol', 'utf8'),
};

var output = solc.compile({ sources: input }, 1)

if (!output) {
    abort('No output from compiler');
} else if (output.errors) {
    for (var error in output.errors) {
        var message = output.errors[error]
        console.error(message)
    }
    abort('Errors occur.');
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

let deployerContract = output.contracts["Deployer.sol:Deployer"]
writeFile(compiledPath + '/Deployer.bin', deployerContract.bytecode);
writeFile(compiledPath + '/Deployer.abi', deployerContract.interface);
