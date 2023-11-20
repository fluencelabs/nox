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

## Configuration

### Nox configuration

Default configuration file is found at
[Config.default.toml](https://github.com/fluencelabs/nox/tree/master/Config.default.toml).

All options can be overwriten by env variables:

```toml
no_banner = false
bootstrap_nodes = [
  "/dns4/0-node.example.com/tcp/9000",
  "/dns4/1-node.example.com/tcp/9000",
]

[system_services]
  [[aqua_ipfs]]
  external_api_multiaddr = "/dns4/ipfs.example.com/tcp/5001"
  local_api_multiaddr = "/dns4/ipfs.service.consul/tcp/5001"
```

becomes

```shell
FLUENCE_NO_BANNER=false
FLUENCE_BOOTSTRAP_NODES="/dns4/0-node.example.com/tcp/9000,/dns4/1-node.example.com/tcp/9000"
FLUENCE_SYSTEM_SERVICES__AQUA_IPFS__EXTERNAL_API_MULTIADDR="/dns4/ipfs.example.com/tcp/5001"
FLUENCE_SYSTEM_SERVICES__AQUA_IPFS__LOCAL_API_MULTIADDR="/dns4/ipfs.service.consul/tcp/5001"
```

### Docker configuration

Some options are only available as env variables:

| var              | default                    | description                                                    |
| ---------------- | -------------------------- | -------------------------------------------------------------- |
| FLUENCE_BASE_DIR | `/.fluence`                | Base directory for nox persistent data                         |
| FLUENCE_CONFIG   | `/.fluence/v1/Config.toml` | Path to nox config file                                        |
| FLUENCE_UID      | `1000`                     | UID of a nox user who owns persistent data                     |
| RUST_LOG         | `info`                     | https://docs.rs/env_logger/0.10.0/env_logger/#enabling-logging |

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

### [trust-graph](https://github.com/fluencelabs/trust-graph)

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

Please, file an [issue](https://github.com/fluencelabs/nox/issues) if you find a
bug. You can also contact us at [Discord](https://discord.com/invite/5qSnPZKh7u)
or [Telegram](https://t.me/fluence_project). We will do our best to resolve the
issue ASAP.

## Contributing

Any interested person is welcome to contribute to the project. Please, make sure
you read and follow some basic
[rules](https://github.com/fluencelabs/nox/tree/master/CONTRIBUTING.md). The
Contributor License Agreement can be found
[here](https://github.com/fluencelabs/nox/tree/master/FluenceCLA).

## License

All software code is copyright (c) Fluence Labs, Inc. under the
[Apache-2.0](https://github.com/fluencelabs/nox/tree/master/LICENSE) license.
