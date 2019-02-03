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

// module.exports = {
//   compilers: {
//     solc: {
//       version: <string>, // A version or constraint - Ex. "^0.5.0"
//                          // Can also be set to "native" to use a native solc
//       docker: <boolean>, // Use a version obtained through docker
//       settings: {
//         optimizer: {
//           enabled: <boolean>,
//           runs: <number>   // Optimize for how many times you intend to run the code
//         }
//         evmVersion: <string> // Default: "byzantium"
//       }
//     }
//   }
// }