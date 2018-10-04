## Running local Fluence clusters

### Prerequisites
* `docker` for running clusters
* `npm` or `python` for connecting clients to them

### Building docker image

Locate to the Fluence root directory and run:

```
sbt docker
```

It would build a `statemachine/statemachine:latest` image.

### Launching clusters

2 example clusters can be found in this directory. Try running the following commands:
```
./run-cluster-counter.sh
./run-cluster-sqldb.sh
```

They would launch the isolated clusters of `statemachine/statemachine:latest` containers.
These clusters share common settings but use different VM code.  

It's possible to attach the running containers, for example:
```
docker attach sqldbnet_node0
```

(Use `Ctrl+P Ctrl+Q` to detach.)

Explore `run-cluster.sh` command to launch clusters with custom VM code. 

### Connecting JS client

Run:

```
npm run start:dev
```

It would launch `npm` project on `localhost:8080`. By opening this URL in a web browser and opening dev console,
it's possible to use client API and demo clients, e. g.:

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