# deploy
Deploy scripts, docker-compose files, and other Fluence DevOps things


To deploy Fluence, Parity and Swarm nodes on remote machines:
- `docker` and `docker-compose` should be installed on remote machines
- Install `fabric 1.14.0` python lib 
- Add your environment in `info.json` with IPs, accounts and ports (see `info.json` for example)
- run `fab --set environment=<env> deploy` and wait

To start it locally with Parity in `dev` mode:
- `docker` and `docker-compose` should be installed on a local machine
- `cd scripts` and run `./compose.sh` and wait
- to start 4 nodes locally use flag `multiple`: `./compose.sh multiple`

# Netdata
## Master
```bash
fab --set environment=ENV,caddy_login=LOGIN,caddy_password=PASSWORD,caddy_port=1234,role=master deploy_netdata
```

## Slave
By default, slaves connect to netdata.fluence.one:19999

```bash
fab --set environment=ENV,caddy_login=LOGIN,caddy_password=PASSWORD,caddy_port=1234,role=slave deploy_netdata
```

## Troubleshooting
If slaves connect to the master, they should be listed in `api/v1/info` on master. i.e.,:
```bash
$ curl netdata.fluence.one:1337/api/v1/info
```
```json
{
	"version": "v1.14.0-rc0-54-g2192f26f",
	"uid": "cac04106-6037-11e9-af0f-0242ac120002",
	"mirrored_hosts": [
		"netdata-master",
		"devnet-06",
		"fluence-node-05",
		"fluence-node-stronger-04",
		"fluence-node-stronger-03",
		"fluence-node-stronger-02"
	],
	"alarms": {
		"normal": 0,
		"warning": 0,
		"critical": 0
	}
}
```

Also, on slaves in logs you will see 'STREAM_SENDER' entries, and 'STREAM' with 'CONNECTED'/'DISCONNECTED' on master.

Slave with hostname `wow-hostname-09`:
```
2019-04-16 11:23:56: netdata INFO  : STREAM_SENDER[wow-hostname-09] : thread created with task id 117
2019-04-16 11:23:56: netdata INFO  : STREAM_SENDER[wow-hostname-09] : STREAM wow-hostname-09 [send]: thread created (task id 117)
2019-04-16 11:24:01: netdata INFO  : STREAM_SENDER[wow-hostname-09] : STREAM wow-hostname-09 [send to netdata.fluence.one:19999]: connecting...
2019-04-16 11:24:01: netdata INFO  : STREAM_SENDER[wow-hostname-09] : STREAM wow-hostname-09 [send to netdata.fluence.one:19999]: initializing communication...
2019-04-16 11:24:01: netdata INFO  : STREAM_SENDER[wow-hostname-09] : STREAM wow-hostname-09 [send to netdata.fluence.one:19999]: waiting response from remote netdata...
2019-04-16 11:24:01: netdata INFO  : STREAM_SENDER[wow-hostname-09] : STREAM wow-hostname-09 [send to netdata.fluence.one:19999]: established communication - ready to send metrics...
```

Master:
```
2019-04-16 11:26:31: STREAM: 416 '[1.2.3.4]:43600' 'CONNECTED' host 'wow-hostname-09' api key ...
2019-04-16 11:26:40: STREAM: 413 '[1.2.3.4]:33080' 'DISCONNECTED' host 'wow-hostname-06' api key ...
2019-04-16 11:26:40: STREAM: 414 '[1.2.3.4]:53164' 'DISCONNECTED' host 'wow-hostname-07' api key ...

```