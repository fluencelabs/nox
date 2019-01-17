## Fluence Prerequisites:
- contract address

## User Prerequisites:
- script to deploy all containers locally
- Fluence CLI
- ethereum account with funds (kovan)
- docker
- external IP

## Node Owner Flow
- download script and fluence CLI (todo: add links in release, script in archive?)
- move CLI to scripts directory `mv fluence path/to/scripts/`
- run `./kovan-compose.sh <your-external-ip> <wallet-address> <secret-key>`
- secret key needed to sign transactions locally
- remember TENDERMINT_KEY from logs, this is the identity of your node
- check that your node is registered here: ...

## Advanced
- if you want to run private node for your own apps add this...
- if you already have started ethereum node or swarm node (or both), use this flags...
- you can register your node as private, so only you can deploy apps to it
- to deploy app use precompiled wasm app here
- use cli for this:
 