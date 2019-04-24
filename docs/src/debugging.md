### Debugging with local Fluence node

Keep your write-test-fix cycle short by debugging your application locally, without waiting for deployment and network.

## Requirements

You will need [Docker](https://docs.docker.com/install/) installed.

## How to

NOTE: For this guide, you will need compiled Wasm code. [Read here](quickstart/backend.md#compiling-to-webassembly) to learn about compilation to Wasm.

### Start your application

Run the following command in terminal, it will start your application in a Docker container:

```
docker run --rm -v /path/to/your/code.wasm:/code/code.wasm -p 30000:30000 fluencelabs/frun
```

To connect to your application from JS client use `directConnect` method:

```javascript
import * as fluence from "fluence";
...
let appId = 1
fluence.directConnect("localhost", 30000, appId);
```
`appId` could be any number. You do not need to specify the correct `appId` for local testing.

NOTE: Use `connect` method instead of `directConnect` for an application deployed in the Fluence network. [Read here](frontend/javascript.md) to learn how to interact with deployed applications.

Send request `val request = session.request("Fluence")` to [hello-world app](https://github.com/fluencelabs/tutorials/tree/master/hello-world) through JavaScript client and we can see in docker logs:
```
[Mon Apr 22 16:03:31 MSK 2019, info, Main$] Tx request. appId: 1
[Mon Apr 22 16:03:31 MSK 2019, info, Main$] Tx: 'Fluence'
INFO  [hello_world2_2018] Fluence has been successfully greeted
```
`Tx: 'Fluence'` is our request, `INFO  [hello_world2_2018] Fluence has been successfully greeted` is logs from code. How to write logs from code code you can see in [backend guides](backend/index.md).

After this get response `request.result()`. And we will see the result:
```
[Mon Apr 22 16:03:31 MSK 2019, info, Main$] Query request. appId: 1, path: sessionId/0, data: Some()
[Mon Apr 22 16:03:31 MSK 2019, info, TxProcessor] Queried result: Hello, world! From user Fluence
```

NOTE: You can also send requests from command line with curl, read [here](http.md) for more.
