var Migrations = artifacts.require("./Migrations.sol");
var Deployer = artifacts.require("./Deployer.sol");

module.exports = function(deployer) {
  deployer.deploy(Deployer);
  deployer.deploy(Migrations);
};
