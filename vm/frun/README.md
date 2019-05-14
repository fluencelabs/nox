# How to run

Given a file `hello_world.wasm`:
```bash
docker run --rm -v /path/to/hello_world.wasm:/code/hello_world.wasm -p 30000:30000 fluencelabs/frun
```

Connect from JS client:
```js
import * as fluence from "fluence";
...
fluence.directConnect("localhost", 30000, 1, "session");
```