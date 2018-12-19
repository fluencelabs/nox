var FluenceContract = artifacts.require("./Network.sol");

module.exports = function(deployer) {
    deployer.deploy(FluenceContract);
};
