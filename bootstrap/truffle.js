module.exports = {
  // See <http://truffleframework.com/docs/advanced/configuration>
  // for more about customizing your Truffle configuration!
  networks: {
    development: {
      host: "0.0.0.0",
      port: 8545,
      network_id: "*" // Match any network id
    }
  },
  mocha: {
    reporter: 'eth-gas-reporter',
    reporterOptions : {
      currency: 'USD',
    }
  },
  compilers: {
    solc: {
      // version: "native",
      settings: {
        optimizer: {
          enabled: true,
          runs: 200,   // Optimize for how many times you intend to run the code
        }
      }
    }
  }
};
