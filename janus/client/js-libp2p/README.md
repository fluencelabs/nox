# Janus browser client
Browser client for the Fluence network based on the js-libp2p.

## How to build

With `npm` installed building could be done as follows:

```bash
npm install
```

## Example 

Shows how to register and call new service in Fluence network

```typescript
interface Server {
    peer: string,
    ip: string,
    port: number
}

function server(peer: string, ip: string, port: number): Server {
    return {
        peer: peer,
        ip: ip,
        port: port
    }
}

const servers = [
    // /ip4/134.209.186.43/tcp/7001 /ip4/134.209.186.43/tcp/9001/ws QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz
    // /ip4/134.209.186.43/tcp/7002 /ip4/134.209.186.43/tcp/9002/ws QmVzDnaPYN12QAYLDbGzvMgso7gbRD9FQqRvGZBfeKDSqW
    // /ip4/134.209.186.43/tcp/7003 /ip4/134.209.186.43/tcp/9003/ws QmSTTTbAu6fa5aT8MjWN922Y8As29KTqBwvvp7CyrC2S6D
    // /ip4/134.209.186.43/tcp/7004 /ip4/134.209.186.43/tcp/9004/ws QmUGQ2ikgcbJUVyaxBPDSWLNUMDo2hDvE9TdRNJY21Eqde
    // /ip4/134.209.186.43/tcp/7005 /ip4/134.209.186.43/tcp/9005/ws Qmdqrm4iHuHPzgeTkWxC8KRj1voWzKDq8MUG115uH2WVSs
    server("QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz", "134.209.186.43", 9001),
    server("QmVzDnaPYN12QAYLDbGzvMgso7gbRD9FQqRvGZBfeKDSqW", "134.209.186.43", 9002),
    server("QmSTTTbAu6fa5aT8MjWN922Y8As29KTqBwvvp7CyrC2S6D", "134.209.186.43", 9003)
];

// Shows how to register and call new service in Fluence network
export async function testCalculator() {
    let key1 = await Janus.generatePeerId();
        let key2 = await Janus.generatePeerId();
    
        // connect to two different nodes
        let cl1 = await Janus.connect("QmSTTTbAu6fa5aT8MjWN922Y8As29KTqBwvvp7CyrC2S6D", "134.209.186.43", 9003, key1);
        let cl2 = await Janus.connect("QmVzDnaPYN12QAYLDbGzvMgso7gbRD9FQqRvGZBfeKDSqW", "134.209.186.43", 9002, key2);
    
        // service name that we will register with one connection and call with another
        let serviceId = "sum-calculator-" + genUUID();
    
        // register service that will add two numbers and send a response with calculation result
        await cl1.registerService(serviceId, async (req) => {
            console.log("message received");
            console.log(req);
    
            console.log("send response");
    
            let message = {msgId: req.arguments.msgId, result: req.arguments.one + req.arguments.two};
    
            await cl1.sendMessage(req.reply_to, message);
        });
    
    
        // msgId is to identify response
        let msgId = "calculate-it-for-me";
    
        let req = {one: 12, two: 23, msgId: msgId};
    
    
        let predicate: (args: any) => boolean | undefined = (args: any) => args.msgId && args.msgId === msgId;
    
        // send call to `sum-calculator` service with two numbers
        let response = await cl2.sendServiceCallWaitResponse(serviceId, req, predicate);
    
        let result = response.result;
        console.log(`calculation result is: ${result}`);
    
        await cl1.connect("QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz", "134.209.186.43", 9001);       
    
        // send call to `sum-calculator` service with two numbers
        await cl2.sendServiceCall(serviceId, req, "calculator request");
    
        let response2 = await cl2.sendServiceCallWaitResponse(serviceId, req, predicate);
    
        let result2 = await response2.result;
        console.log(`calculation result AFTER RECONNECT is: ${result2}`);
}
```

