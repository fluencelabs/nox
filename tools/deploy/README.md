# deploy
Deploy scripts, docker-compose files, and other Fluence DevOps things


To deploy Fluence, Parity and Swarm nodes on remote machines:
- `docker` and `docker-compose` should be installed on remote machines
- Install `fabric` lib
- Change hosts, addresses, keys, contract address if needed
- run `fab deploy` and wait

To start it locally with Parity in `dev` mode:
- `docker` and `docker-compose` should be installed on a local machine
- run `./scripts/compose.sh` and wait
 