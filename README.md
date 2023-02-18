# Rust Peer

Rust Peer is the reference implementation of the [Fluence](https://fluence.network) peer. It is used as a Relay for all Clients and as a Host for all Workers.


## Installation and Usage

Rust peer is distributed as a [docker image](https://github.com/fluencelabs/rust-peer-distro/releases). To start a local instance of Rust peer, run:

```bash
docker pull fluencelabs/rust-peer:latest
docker run -d --name fluence -e RUST_LOG="info" -p 7777:7777 -p 9999:9999 fluencelabs/rust-peer:latest --local --keypair-value=gKdiCSUr1TFGFEgu2t8Ch1XEUsrN5A2UfBLjSZvfci9SPR3NvZpACfcpPGC3eY4zma1pk7UvYv5zb1VjvPHwCjj
```

This will setup a network of one Rust peer and an IPFS sidecar, not connected to any other network. Next, run some [Aqua](https://github.com/fluencelabs/aqua) against it:

```bash
npm i -g @fluencelabs/aqua@unstable
aqua remote list_modules --addr /ip4/127.0.0.1/tcp/9999/ws/p2p/12D3KooWKEprYXUXqoV5xSBeyqrWLpQLLH4PXfvVkDJtmcqmh5V3
```

For more info about the docker image (image version flavours, environment variables, deployment examples) and documentation for Rust peer operators, see the [rust-peer-distro](https://github.com/fluencelabs/rust-peer-distro) repository.


## Documentation

Comprehensive documentation on everything related to Fluence can be found [here](https://fluence.dev/). Check also our [YouTube channel](https://www.youtube.com/@fluencelabs).


## Support

Please, file an [issue](https://github.com/fluencelabs/rust-peer/issues) if you find a bug. You can also contact us at [Discord](https://discord.com/invite/5qSnPZKh7u) or [Telegram](https://t.me/fluence_project).  We will do our best to resolve the issue ASAP.


## Contributing

Any interested person is welcome to contribute to the project. Please, make sure you read and follow some basic [rules](./CONTRIBUTING.md). The Contributor License Agreement can be found [here](./FluenceCLA).


## License

All software code is copyright (c) Fluence Labs, Inc. under the [Apache-2.0](./LICENSE) license.

