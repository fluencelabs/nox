## Running local Fluence clusters

### Prerequisites
* `docker` and `jq` for running clusters
* `npm` or `python` for connecting clients to them

### Building docker image

Locate to the Fluence root directory and run:

```
sbt docker
```

It would build a `fluencelabs/solver:latest` image.

### Launching clusters

2 example clusters can be found in `examples` directory. Try running the following commands:
```
./sim-cluster-counter.sh
./sim-cluster-sqldb.sh
```

They would launch the isolated clusters of `fluencelabs/statemachine:latest` containers.
These clusters share common settings but use different VM code.  

It's possible to attach to the running containers, for example:
```
docker attach sqldbnet_node0
```

It would allow to view State machine logs reporting details about blocks, transactions, etc. (Use `Ctrl+P Ctrl+Q` to detach.)

Explore `sim-cluster.sh` command to launch clusters with custom VM code. 

### Connecting JS client

Locate to `<fluence root>/js-client` directory and run:

```
npm run start:dev
```

It would launch `npm` project on `localhost:8080`. By opening this URL in a web browser and opening dev console,
it's possible to use the client API and demo client implementations, e. g.:

```
# counter cluster needs to be launched 
var imClient = new IncrementAndMultiply("localhost", 25057)
await imClient.multiply(10, 5);

# sqldb cluster needs to be launched 
var dbClient = new DbOnPointers("localhost", 27057);
await dbClient.test();
```   

### Connecting Python client

To connect `counter` cluster and send some example commands, locate to `<fluence root>/client/example` and run:
```
python dataengine-example.py 25057
```