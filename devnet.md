# Devnet Overview

## Roles

Roles overview [readme](docs/src/introduction.md).

- Miner ([guide](docs/src/miner.md))
- Backend Developer ([guide](docs/src/backend.md))
- Frontend Developer ([guide](docs/src/frontend.md))

## Moving Parts

- Smart Contract ([readme](bootstrap/README.md))
- CLI tool ([readme](cli/README.md))
- Swarm integration ([readme](externalstorage/README.md))
- JS client ([readme](fluence-js/README.md), [npm](https://www.npmjs.com/package/fluence-js))
- Node ([docker hub](https://hub.docker.com/r/fluencelabs/node/))
- Worker ([readme](statemachine/README.md), [docker hub](https://hub.docker.com/r/fluencelabs/worker))
- VM ([examples readme](vm/examples/README.md), [sdk readme](sdk/rust/README.md), [backend guide](docs/src/backend.md))
- DevOps ([readme](tools/deploy/README.md))
- Prometheus/Grafana monitoring ([readme](tools/monitoring/README.md)) _(obsolete)_
- Dashboard ([readme](monitoring/README.md), [deployed](http://142.93.108.175:8080/))

## Feature matrix

|                         | Smart Contract |      CLI      | Swarm Interop |   JS client   |     Node      |    Worker     |      VM       |    DevOps     |   Dashboard   |
| :---                    |     :---:      |     :---:     |     :---:     |     :---:     |     :---:     |     :---:     |     :---:     |     :---:     |     :---:     |
| Miner: addNode          |     **Y**      |     **Y**     |       -       |       -       |     **Y**     |       -       |       -       |     **Y**     |     **Y**     |
| Miner: removeNode       |     **Y**      |     **Y**     |       -       |       -       |     **Y**     |     **Y**     |       -       |       -       |     **Y**     |
| Back: addApp            |     **Y**      |     **Y**     |     **Y**     |       -       |     **Y**     |     **Y**     |       -       |       -       |     **Y**     |
| Back: removeApp         |     **Y**      |     **Y**     |       -       |       -       |     **Y**     |     **Y**     |       -       |       -       |     **Y**     |
| Back: develop App       |       -        |       ?       |       -       |       -       |       -       |       -       |     **Y**     |       -       |       -       |
| Front: use App          |     **Y**      |       -       |       -       |     **Y**     |     **Y**     |     **Y**     |     **Y**     |       -       |       -       |

## Missing parts

- Staging network
- VM
    - Protocol is unstable: move ordering inside Rust SDK
    - Merkle state hash
    - Verification game
- Swarm
    - Store Workers TX
    - We need to have Receipts (Swarm Mainnet)
- Smart Contract
    - Prove Tendermint Validator Key ownership (Constantinople)    
- Dashboard
    - UI    
