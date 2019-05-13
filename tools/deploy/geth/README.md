# Running Geth with balancer
## Services involved
- Geth itself, on :8545, to give access to the Ethereum network
    - Also there is :30303 for 3rd-party Geth nodes (or light clients) to connect to this one
- Caddy, on :8546, does the load balancing
- Nginx, translates HTTP GET on localhost:20000/sync to HTTP POST sending `eth_syncing` to local Geth
    - connected to the Geth container via `external_links` in geth.yml

Caddy expects requests to arrive to `http://geth.fluence.one:8546`

## How to run
### Geth nodes
On all Geth nodes, copy:
 - geth.yml
 - endpoint.nginx.conf
 
and run `docker-compose -f geth.yml up -d`

### Balancer node
Copy:
- caddy.yml
- Caddyfile

and run `docker-compose -f caddy.yml up -d`

## Adding new nodes to the balancer
To add a new Geth node, simply add it's address (domain or IP) to Caddyfile to the `proxy` statement.

That's it. Now, restart caddy via `docker-compose -f caddy.yml restart`, and check it works.
    