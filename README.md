# Nox

Nox is the reference implementation of the [Fluence](https://fluence.network)
peer. It is used as a Relay for all Clients and as a Host for all Workers.

## Installation and Usage

nox is distributed as a
[docker image](https://hub.docker.com/r/fluencelabs/nox). To start a local
instance of nox, run:

```bash
docker pull fluencelabs/nox:latest
docker run -d --name fluence -e RUST_LOG="info" -p 7777:7777 -p 9999:9999 fluencelabs/nox:latest --local
```

This will setup a network of one nox not connected to any other network.

For more info about the docker image see the
[README](https://github.com/fluencelabs/nox/blob/master/docker/README.md).

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
you read and follow some basic [rules](./CONTRIBUTING.md). The Contributor
License Agreement can be found [here](./FluenceCLA).

## License

All software code is copyright (c) Fluence Labs, Inc. under the
[Apache-2.0](./LICENSE) license.
