# Fluence HTTP protocol

Fluence HTTP protocol is simple, and supports two main features: session-handling, and request ordering.

Given IP `1.2.3.4`, request with a tx would look like this

```
curl 'http://1.2.3.4:25000/apps/2/tx' --data $'sessionId/0\nRequestBody'
```

- `:25000` - default Fluence port
- `apps` - prefix for appId routing
- `2` - appId
- `tx` - suffix for all txs
- `sessionId` - random client-generated string, requests are ordered by that string
- `0` - requests counter, user for ordering, should be incremented sequentially, i.e., avoid number skipping
- '\n'- separator between `sessionId/counter` and the request
- 'RequestBody' - could be anything, sent directly to your application

So, the main thing to take away is a `sessionId/counter\nbody` format, and remember to increment requests counter sequentially, because Fluence will wait for all omitted request numbers. This is done to preserve strict ordering, so Fluence can guarantee request processing in the strict order.
