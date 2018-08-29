var Deployer = artifacts.require("./Deployer.sol");

module.exports = function(deployer) {
    deployer.deploy(Deployer);
};
