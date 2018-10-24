## Monitoring local Fluence clusters via Prometheus and Grafana

### Prerequisites
* `docker` and `docker-compose`

### Launching monitoring tools

Run from `tools/monitoring` directory:

```
docker-compose up
```

It would run `prometheus` on [localhost:9090](localhost:9090) and `grafana` on [localhost:3000](localhost:3000).
Use `admin` user with `foobar` password to login in Grafana.

### Stopping monitoring tools

```
docker-compose down
```