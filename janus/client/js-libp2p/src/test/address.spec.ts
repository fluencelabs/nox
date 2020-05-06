import {
    createPeerAddress,
    createRelayAddress,
    createServiceAddress,
    addressToString,
    parseAddress
} from "../address";
import {expect, assert} from 'chai';

import 'mocha';
import * as PeerId from "peer-id";
import {callToString, genUUID, makeFunctionCall, parseFunctionCall} from "../function_call";
import Janus from "../janus";

describe("Typescript usage suite", () => {

    it("should throw an error, if protocol will be without value", () => {
        expect(() => parseAddress("/peer/")).to.throw(Error);
    });

    it("should be able to convert service_id address to and from string", () => {
        let addr = createServiceAddress("service_id-1");
        let str = addressToString(addr);
        let parsed = parseAddress(str);

        expect(parsed).to.deep.equal(addr)
    });

    it("should be able to convert peer address to and from string", () => {
        let pid = PeerId.createFromB58String("QmXduoWjhgMdx3rMZXR3fmkHKdUCeori9K1XkKpqeF5DrU");
        let addr = createPeerAddress(pid.toB58String());
        let str = addressToString(addr);
        let parsed = parseAddress(str);

        expect(parsed).to.deep.equal(addr)
    });

    it("should be able to convert relay address to and from string", async () => {
        let pid = await PeerId.create();
        let relayid = await PeerId.create();
        let addr = await createRelayAddress(relayid.toB58String(), pid, true);
        let str = addressToString(addr);
        let parsed = parseAddress(str);

        expect(parsed).to.deep.equal(addr)
    });

    it("should be able to convert function call to and from string", async () => {
        let pid = await PeerId.create();
        let relayid = await PeerId.create();
        let addr = await createRelayAddress(relayid.toB58String(), pid, true);

        let pid2 = await PeerId.create();
        let addr2 = createPeerAddress(pid.toB58String());

        let functionCall = makeFunctionCall(
            "123",
            addr2,
            {
                arg1: "123",
                arg2: 3,
                arg4: [1, 2, 3]
            },
            addr,
            "2444"
        );

        let str = callToString(functionCall);

        let parsed = parseFunctionCall(str);

        expect(parsed).to.deep.equal(functionCall);

        let functionCallWithOptional = makeFunctionCall(
            "123",
            addr,
            {
                arg1: "123",
                arg2: 3,
                arg4: [1, 2, 3]
            }
        );

        let str2 = callToString(functionCallWithOptional);

        let parsed2 = parseFunctionCall(str2);

        expect(parsed2).to.deep.equal(functionCallWithOptional)

    });

    it("integration test", async function () {
        this.timeout(5000);
        await testCalculator();
    });
});

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
    // /ip4/104.248.25.59/tcp/7001 /ip4/104.248.25.59/tcp/9001/ws 12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9
    // /ip4/104.248.25.59/tcp/7002 /ip4/104.248.25.59/tcp/9002/ws 12D3KooWHk9BjDQBUqnavciRPhAYFvqKBe4ZiPPvde7vDaqgn5er
    // /ip4/104.248.25.59/tcp/7003 /ip4/104.248.25.59/tcp/9003/ws 12D3KooWBUJifCTgaxAUrcM9JysqCcS4CS8tiYH5hExbdWCAoNwb
    // /ip4/104.248.25.59/tcp/7004 /ip4/104.248.25.59/tcp/9004/ws 12D3KooWJbJFaZ3k5sNd8DjQgg3aERoKtBAnirEvPV8yp76kEXHB
    // /ip4/104.248.25.59/tcp/7005 /ip4/104.248.25.59/tcp/9005/ws 12D3KooWCKCeqLPSgMnDjyFsJuWqREDtKNHx1JEBiwaMXhCLNTRb
    // Bootstrap:
// /ip4/104.248.25.59/tcp/7770 /ip4/104.248.25.59/tcp/9990/ws 12D3KooWMhVpgfQxBLkQkJed8VFNvgN4iE6MD7xCybb1ZYWW2Gtz
// Special ones:
//     /ip4/104.248.25.59/tcp/7100 /ip4/104.248.25.59/tcp/9100/ws 12D3KooWPnLxnY71JDxvB3zbjKu9k1BCYNthGZw6iGrLYsR1RnWM
    server("QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz", "104.248.25.59", 9001),
    server("QmVzDnaPYN12QAYLDbGzvMgso7gbRD9FQqRvGZBfeKDSqW", "104.248.25.59", 9002),
    server("QmSTTTbAu6fa5aT8MjWN922Y8As29KTqBwvvp7CyrC2S6D", "104.248.25.59", 9003)
    // /ip4/104.248.25.59/tcp/7001 /ip4/104.248.25.59/tcp/9001/ws QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz
    // /ip4/104.248.25.59/tcp/7002 /ip4/104.248.25.59/tcp/9002/ws QmVzDnaPYN12QAYLDbGzvMgso7gbRD9FQqRvGZBfeKDSqW
    // /ip4/104.248.25.59/tcp/7003 /ip4/104.248.25.59/tcp/9003/ws QmSTTTbAu6fa5aT8MjWN922Y8As29KTqBwvvp7CyrC2S6D
];

const delay = (ms: number) => new Promise(res => setTimeout(res, ms));

// Shows how to register and call new service in Fluence network
export async function testCalculator() {

    let key1 = await Janus.generatePeerId();
    let key2 = await Janus.generatePeerId();

    // connect to two different nodes
    let cl1 = await Janus.connect("12D3KooWBUJifCTgaxAUrcM9JysqCcS4CS8tiYH5hExbdWCAoNwb", "104.248.25.59", 9003, key1);
    let cl2 = await Janus.connect("12D3KooWHk9BjDQBUqnavciRPhAYFvqKBe4ZiPPvde7vDaqgn5er", "104.248.25.59", 9002, key2);

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

    await cl1.connect("12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9", "104.248.25.59", 9001);

    await delay(1000);

    // send call to `sum-calculator` service with two numbers
    await cl2.sendServiceCall(serviceId, req, "calculator request");

    let response2 = await cl2.sendServiceCallWaitResponse(serviceId, req, predicate);

    let result2 = await response2.result;
    console.log(`calculation result AFTER RECONNECT is: ${result2}`);
}
