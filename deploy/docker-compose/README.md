# Run nox with docker-compose

This guide explains how to use docker-compose to start a local network of three
[nox](https://github.com/fluencelabs/nox) nodes.

## Introduction

By running a local nox network, you can test your applications in a controlled
environment without relying on external networks.

The nox network is a set of peer nodes that can communicate with each other to
share data and execute code, local IPFS instance used by
[aqua-ipfs builtin](https://github.com/fluencelabs/aqua-ipfs) and a blockchain
node used by [decider builtin](https://github.com/fluencelabs/decider).

## Prerequisites

Before you can run the nox network, you need to have Docker and docker-compose
installed on your system. You can follow the official instructions for
installing Docker and installing docker-compose on your operating system:

- [docker](https://docs.docker.com/engine/install/)
- [docker-compose](https://docs.docker.com/compose/install/linux/#install-using-the-repository)

## Starting local nox network

1. `git clone` this repository locally and run `cd deploy/docker-compose`.

2. Pull the latest container images by running the following command:
   ```bash
   docker-compose pull
   ```

3. Start the nox network by running the following command:
   ```bash
   docker-compose up -d
   ```

This will start three nox nodes, each listening on a different port.

## Accessing local nox network

To interact with the nox network, you can use the
[fluence-cli](https://github.com/fluencelabs/cli) tool.

1. Run `fluence init` and chose `minimal` project template.
2. Change `hosts` key in `fluence.yaml` to:
   ```yml
   hosts:
     defaultWorker:
       peerIds:
         - 12D3KooWBM3SdXWqGaawQDGQ6JprtwswEg3FWGvGhmgmMez1vRbR
   ```

3. Change `relays` key in `fluence.yaml` to:
   ```yml
   relays:
     - /ip4/127.0.0.1/tcp/9991/ws/p2p/12D3KooWBM3SdXWqGaawQDGQ6JprtwswEg3FWGvGhmgmMez1vRbR
     - /ip4/127.0.0.1/tcp/9992/ws/p2p/12D3KooWQdpukY3p2DhDfUfDgphAqsGu5ZUrmQ4mcHSGrRag6gQK
     - /ip4/127.0.0.1/tcp/9993/ws/p2p/12D3KooWRT8V5awYdEZm6aAV9HWweCEbhWd7df4wehqHZXAB7yMZ
   ```

4. Run
   ```bash
   fluence run -f 'helloWorld("Fluence")'
   fluence run -f 'getInfo()'
   ```

## Using local nox network in your project

You must make changes to `fluence.yaml` to use a local nox network:

- changing `hosts` key in `fluence.yaml` to:
  ```yml
  hosts:
    defaultWorker:
      peerIds:
        - 12D3KooWBM3SdXWqGaawQDGQ6JprtwswEg3FWGvGhmgmMez1vRbR
  ```
- changing `relays` key in `fluence.yaml` to:
  ```yml
  relays:
    - /ip4/127.0.0.1/tcp/9991/ws/p2p/12D3KooWBM3SdXWqGaawQDGQ6JprtwswEg3FWGvGhmgmMez1vRbR
    - /ip4/127.0.0.1/tcp/9992/ws/p2p/12D3KooWQdpukY3p2DhDfUfDgphAqsGu5ZUrmQ4mcHSGrRag6gQK
    - /ip4/127.0.0.1/tcp/9993/ws/p2p/12D3KooWRT8V5awYdEZm6aAV9HWweCEbhWd7df4wehqHZXAB7yMZ
  ```

You can try following the example
[workflow](https://github.com/fluencelabs/cli/blob/main/docs/EXAMPLE.md)
provided by Fluence Labs making these changes.

Here is a table with multiaddress for each node:

| container | multiaddress                                                                        |
| --------- | ----------------------------------------------------------------------------------- |
| peer-1    | /ip4/127.0.0.1/tcp/9991/ws/p2p/12D3KooWBM3SdXWqGaawQDGQ6JprtwswEg3FWGvGhmgmMez1vRbR |
| peer-2    | /ip4/127.0.0.1/tcp/9992/ws/p2p/12D3KooWQdpukY3p2DhDfUfDgphAqsGu5ZUrmQ4mcHSGrRag6gQK |
| peer-3    | /ip4/127.0.0.1/tcp/9993/ws/p2p/12D3KooWRT8V5awYdEZm6aAV9HWweCEbhWd7df4wehqHZXAB7yMZ |

## Running with observability stack

Stack consists of:

- [Prometheus](https://prometheus.io/) - TSDB that collects and stores metrics
- [Loki](https://grafana.com/logs/) - lightweight centralized logging solution
- [Promtail](https://grafana.com/docs/loki/latest/clients/promtail/) - log
  collection agent
- [Grafana](https://grafana.com/grafana/) - data visualization tool

To set it up run:

```bash
docker-compose -f docker-compose.yml -f docker-compose.observability.yml up -d
```

Grafana will have automatically preprovisioned dashboards:

- nox stats - overview of nox network
- Service metrics - detailed stats on deployed services

You can find Grafana at http://localhost:3000. To access nox logs use `Explore`
tab and chose `Loki` datasource.
