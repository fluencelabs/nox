import {
    createPeerAddress,
    createRelayAddress,
    createServiceAddress,
    addressToString,
    parseAddress, Address
} from "../address";
import {expect} from 'chai';

import 'mocha';
import {decode, encode} from "bs58"
import * as PeerId from "peer-id";
import {callToString, genUUID, makeFunctionCall, parseFunctionCall} from "../function_call";
import Fluence from "../fluence";
import {certificateFromString, certificateToString, issue} from "../trust/certificate";
import {TrustGraph} from "../trust/trust_graph";
import {nodeRootCert} from "../trust/misc";
import {peerIdToSeed, seedToPeerId} from "../seed";

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

        let addr2 = createPeerAddress(pid.toB58String());

        let functionCall = makeFunctionCall(
            "123",
            addr2,
            addr2,
            {
                arg1: "123",
                arg2: 3,
                arg4: [1, 2, 3]
            },
            "mm",
            "fff",
            addr,
            undefined,
            "2444"
        );

        let str = callToString(functionCall);

        let parsed = parseFunctionCall(str);

        expect(parsed).to.deep.equal(functionCall);

        let functionCallWithOptional = makeFunctionCall(
            "123",
            addr,
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

    it("should create private key from seed and back", async function () {
        let seed = [46, 188, 245, 171, 145, 73, 40, 24, 52, 233, 215, 163, 54, 26, 31, 221, 159, 179, 126, 106, 27, 199, 189, 194, 80, 133, 235, 42, 42, 247, 80, 201];
        let seedStr = encode(seed)
        console.log("SEED STR: " + seedStr)
        let pid = await seedToPeerId(seedStr)
        expect(peerIdToSeed(pid)).to.be.equal(seedStr)
    })

    it("should serialize and deserialize certificate correctly", async function () {
        let cert = `11
1111
5566Dn4ZXXbBK5LJdUsE7L3pG9qdAzdPY47adjzkhEx9
3HNXpW2cLdqXzf4jz5EhsGEBFkWzuVdBCyxzJUZu2WPVU7kpzPjatcqvdJMjTtcycVAdaV5qh2fCGphSmw8UMBkr
158981172690500
1589974723504
2EvoZAZaGjKWFVdr36F1jphQ5cW7eK3yM16mqEHwQyr7
4UAJQWzB3nTchBtwARHAhsn7wjdYtqUHojps9xV6JkuLENV8KRiWM3BhQByx5KijumkaNjr7MhHjouLawmiN1A4d
1590061123504
1589974723504`

        let deser = await certificateFromString(cert);
        let ser = certificateToString(deser);

        expect(ser).to.be.equal(cert);
    });

    // delete `.skip` and run `npm run test` to check service's and certificate's api with Fluence nodes
    it("integration test", async function () {
        this.timeout(15000);
        // await testCerts();
        // await testCalculator();
        await testGetFullServicesInfo();
    });
});

const delay = (ms: number) => new Promise(res => setTimeout(res, ms));

export async function testCerts() {
    let key1 = await Fluence.generatePeerId();
    let key2 = await Fluence.generatePeerId();

    // connect to two different nodes
    let cl1 = await Fluence.connect("/dns4/134.209.186.43/tcp/9003/ws/p2p/12D3KooWBUJifCTgaxAUrcM9JysqCcS4CS8tiYH5hExbdWCAoNwb", key1);
    let cl2 = await Fluence.connect("/ip4/134.209.186.43/tcp/9002/ws/p2p/12D3KooWHk9BjDQBUqnavciRPhAYFvqKBe4ZiPPvde7vDaqgn5er", key2);

    let trustGraph1 = new TrustGraph(cl1);
    let trustGraph2 = new TrustGraph(cl2);

    let issuedAt = new Date();
    let expiresAt = new Date();
    // certificate expires after one day
    expiresAt.setDate(new Date().getDate() + 1);

    // create root certificate for key1 and extend it with key2
    let rootCert = await nodeRootCert(key1);
    let extended = await issue(key1, key2, rootCert, expiresAt.getTime(), issuedAt.getTime());

    // publish certificates to Fluence network
    await trustGraph1.publishCertificates(key2.toB58String(), [extended]);

    // get certificates from network
    let certs = await trustGraph2.getCertificates(key2.toB58String());

    // root certificate could be different because nodes save trusts with bigger `expiresAt` date and less `issuedAt` date
    expect(certs[0].chain[1].issuedFor.toB58String()).to.be.equal(extended.chain[1].issuedFor.toB58String())
    expect(certs[0].chain[1].signature).to.be.equal(extended.chain[1].signature)
    expect(certs[0].chain[1].expiresAt).to.be.equal(extended.chain[1].expiresAt)
    expect(certs[0].chain[1].issuedAt).to.be.equal(extended.chain[1].issuedAt)
}

export async function testGetFullServicesInfo() {
    let key1 = await Fluence.generatePeerId();
    let key2 = await Fluence.generatePeerId();

    // connect to two different nodes
    let cl1 = await Fluence.connect("/dns4/134.209.186.43/tcp/9100/ws/p2p/12D3KooWPnLxnY71JDxvB3zbjKu9k1BCYNthGZw6iGrLYsR1RnWM", key1);
    let cl2 = await Fluence.connect("/ip4/134.209.186.43/tcp/9002/ws/p2p/12D3KooWHk9BjDQBUqnavciRPhAYFvqKBe4ZiPPvde7vDaqgn5er", key2);

    let peerId1 = "12D3KooWPnLxnY71JDxvB3zbjKu9k1BCYNthGZw6iGrLYsR1RnWM"
    let peerAddr1 = createPeerAddress(peerId1);
    let peerAddr2 = createPeerAddress("12D3KooWHk9BjDQBUqnavciRPhAYFvqKBe4ZiPPvde7vDaqgn5er");

    // await cl2.createService(peerAddr1, ["ipfs_node.wasm"]);

    /*let resp = await cl2.callService(peerId1, "9adf40a2-0a4c-43ec-b4d2-305fd762cb46", "ipfs_node.wasm", {}, "get_address")
    console.log(resp)

    let resp2 = await cl1.callService(peerId1, "9adf40a2-0a4c-43ec-b4d2-305fd762cb46", "ipfs_node.wasm", {}, "get_address")
    console.log(resp2)*/

    let r1 = await cl1.getActiveInterfaces();
    let r11 = await cl1.getInterface('9adf40a2-0a4c-43ec-b4d2-305fd762cb46');
    console.log("111111111111111111");
    console.log(r1);
    console.log(r11);
    /*let r2 = await cl1.getActiveInterfaces(peerAddr2);
    console.log("2222222222222222222");
    console.log(r2);*/

    /*let r3 = await cl2.getActiveInterfaces();
    console.log("33333333333333333333");
    console.log(r3);
    let r4 = await cl2.getActiveInterfaces(peerAddr1);
    console.log("4444444444444444444444");
    console.log(r4);*/

    /*let r5 = await cl1.getAvailableModules();
    console.log("111111111111111111");
    console.log(r5);
    let r6 = await cl1.getAvailableModules(peerAddr2);
    console.log("2222222222222222222");
    console.log(r6);

    let r7 = await cl2.getAvailableModules();
    console.log("33333333333333333333");
    console.log(r7);
    let r8 = await cl2.getAvailableModules(peerAddr1);
    console.log("4444444444444444444444");
    console.log(r8);*/
}

// Shows how to register and call new service in Fluence network
export async function testCalculator() {

    let key1 = await Fluence.generatePeerId();
    let key2 = await Fluence.generatePeerId();

    // connect to two different nodes
    let cl1 = await Fluence.connect("/dns4/134.209.186.43/tcp/9003/ws/p2p/12D3KooWBUJifCTgaxAUrcM9JysqCcS4CS8tiYH5hExbdWCAoNwb", key1);
    let cl2 = await Fluence.connect("/ip4/134.209.186.43/tcp/9002/ws/p2p/12D3KooWHk9BjDQBUqnavciRPhAYFvqKBe4ZiPPvde7vDaqgn5er", key2);

    // service name that we will register with one connection and call with another
    let serviceId = "sum-calculator-" + genUUID();

    // register service that will add two numbers and send a response with calculation result
    await cl1.provideName(serviceId, async (req) => {
        console.log("message received");
        console.log(req);

        console.log("send response");

        let message = {msgId: req.arguments.msgId, result: req.arguments.one + req.arguments.two};

        await cl1.sendCall(req.reply_to, message);
    });

    let req = {one: 12, two: 23};

    // send call to `sum-calculator` service with two numbers
    let response = await cl2.callProvider(serviceId, req);

    let result = response.result;
    console.log(`calculation result is: ${result}`);

    await cl1.connect("/dns4/relay02.fluence.dev/tcp/19001/wss/p2p/12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9");

    await delay(1000);

    // send call to `sum-calculator` service with two numbers
    await cl2.callProvider(serviceId, req, "calculator request");

    let response2 = await cl2.callProvider(serviceId, req);

    let result2 = await response2.result;
    console.log(`calculation result AFTER RECONNECT is: ${result2}`);
}

