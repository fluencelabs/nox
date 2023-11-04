## Installation and usage

```bash
docker run -d --name nox -e RUST_LOG="info" -p 7777:7777 -p 9999:9999 fluencelabs/nox:latest --local
```

To get a list of commands that can be passed to nox run:

```bash
docker run --rm --name nox fluencelabs/nox:latest --help
```

See deployment instructions and tips at
[deploy](https://github.com/fluencelabs/nox/tree/master/deploy).

## Builtin services

Nox distro comes with preconfigured builtin services.

### [registry](https://github.com/fluencelabs/registry)

Registry implements service discovery.

### [aqua-ipfs](https://github.com/fluencelabs/aqua-ipfs)

This is a native IPFS integration with
[Aqua](https://fluence.dev/docs/aqua-book/introduction) language. It is used to
orchestrate IPFS file transfer with Aqua scripts.

By default connects to an IPFS daemon hosted by
[Fluence Labs](https://fluence.network).

In case you want to use a separately running IPFS daemon, you need to configure
aqua-ipfs:

```toml
[system_services]
  [[aqua_ipfs]]
  # IPFS multiaddr advertised to clients (e.g., frontend apps) to use in uploading files (ipfs.put), managing pins (ipfs.pin) etc
  external_api_multiaddr = "/dns4/ipfs.fluence.dev/tcp/5001"
  # used by the aqua-ipfs builtin to configure IPFS (bad bad bad)
  local_api_multiaddr = "/dns4/ipfs.fluence.dev/tcp/5001"
```

## [trust-graph](https://github.com/fluencelabs/trust-graph)

It can be used to create a trusted network, to manage service permissions with
TLS certificates and other security related things.

### [decider and connector](https://github.com/fluencelabs/decider)

Used to pull contracts and deploy from blockchain. TODO documentation on
configuration variables

## Documentation

Comprehensive documentation on everything related to Fluence can be found
[here](https://fluence.dev/). Check also our
[YouTube channel](https://www.youtube.com/@fluencelabs).

## Support

Please, file an [issue](https://github.com/fluencelabs/nox-distro/issues) if you
find a bug. You can also contact us at
[Discord](https://discord.com/invite/5qSnPZKh7u) or
[Telegram](https://t.me/fluence_project). We will do our best to resolve the
issue ASAP.

## Contributing

Any interested person is welcome to contribute to the project. Please, make sure
you read and follow some basic
[rules](https://github.com/fluencelabs/nox-distro/tree/master/CONTRIBUTING.md).
The Contributor License Agreement can be found
[here](https://github.com/fluencelabs/nox-distro/tree/master/FluenceCLA).

## License

All software code is copyright (c) Fluence Labs, Inc. under the
[Apache-2.0](https://github.com/fluencelabs/nox-distro/tree/master/LICENSE)
license.