### Debugging with local Fluence node

Deploying your backend code to Fluence network is not necessary if you want to check the correctness of your code.

## Requirements

[docker](https://docs.docker.com/install/)

## How to

If you already have compiled backend code, you can deploy it to Fluence local node for debugging purposes. If you don't have `wasm` file, check [other guides about developing on Fluence](SUMMARY.md). All you need is to run this command in a new terminal window:

```
docker run --rm -v /path/to/your/code.wasm:/code/code.wasm -p 30000:30000 fluencelabs/frun
```

To connect to this code from JS client use `directConnect` method instead of `connect`:

```javascript
import * as fluence from "fluence";
...
let appId = 1
fluence.directConnect("localhost", 30000, appId);
```
`appId` could be any number. You do not need to specify the correct `appId` for local testing.

Also, we can send requests directly from the terminal with `curl` command. Here it is considered in detail: [Fluence HTTP protocol](http.md)

Send request `val request = session.request("Fluence")` to [hello-world app](https://github.com/fluencelabs/tutorials/tree/master/hello-world) through JavaScript client and we can see in docker logs:
```
[blaze-selector-0-1] INFO org.http4s.blaze.channel.nio1.NIO1SocketServerGroup - Accepted connection from /0:0:0:0:0:0:0:1:52896
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
