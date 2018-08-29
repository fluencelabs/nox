module.exports = {
  // See <http://truffleframework.com/docs/advanced/configuration>
  // for more about customizing your Truffle configuration!
  networks: {
    development: {
      host: "127.0.0.1",
      port: 8545,
      from: "0x96dce7eb99848e3332e38663a1968836ba3c3b53",
      gas: 4600000,
      network_id: "1234" // Match any network id
    }
  }
};
