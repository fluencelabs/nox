# Janus browser client
Browser client for the Fluence network based on the js-libp2p.

## How to build

With `npm` installed building could be done as follows:

```bash
npm install
```

## Example 

Shows how to register and call new service in Fluence network

```javascript
const {genUUID} = require("janus-beta/dist/function_call");
const JanusClient = require("janus-beta/dist/janus");
const {makeFunctionCall} = require("janus-beta/dist/function_call");

async function calculateSum() {

    // connect to two different nodes
    let con1 = await JanusClient.connect("QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz", "104.248.25.59", 9001);
    let con2 = await JanusClient.connect("QmVzDnaPYN12QAYLDbGzvMgso7gbRD9FQqRvGZBfeKDSqW", "104.248.25.59", 9002);

    // service name that we will register with one connection and call with another
    let serviceName = "sum-calculator";

    // register service that will add two numbers and send a response with calculation result
    await con1.registerService(serviceName, async (req) => {
        console.log("message received");
        console.log(req);

        console.log("send response");

        let response = makeFunctionCall(genUUID(), req.reply_to, {msgId: req.arguments.msgId, result: req.arguments.one + req.arguments.two});
        console.log(response);

        await con1.sendFunctionCall(response);
    });


    // msgId is to identify response
    let msgId = "calculate-it-for-me";

    let req = {one: 12, two: 23, msgId: msgId};

    // send call to `sum-calculator` service with two numbers
    await con2.sendServiceCall(serviceName, req, "calculator request");

    let resultPromise = new Promise((resolve, reject) => {
        // subscribe for responses, to handle response
        con2.subscribe((call) => {
            if (call.arguments.msgId && call.arguments.msgId === msgId) {
                console.log("response received!");

                resolve(call.arguments.result);
                return true;
            }
            return false;
        });
    });

    let result = await resultPromise;
    console.log(`calculation result is: ${result}`);
}

(async () => {
    await calculateSum().catch((e) => {
        console.log(e)
    });
})();
```

