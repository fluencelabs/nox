import {createPeerAddress, createRelayAddress, createServiceAddress, parseAddress} from "../address";
import {expect} from 'chai';

import 'mocha';
import * as PeerId from "peer-id";
import {FunctionCall, makeFunctionCall, parseFunctionCall} from "../function_call";
import {connect} from "../janus";
import {calcHash} from "../ipfs";

describe("Typescript usage suite", () => {

    it("should be able to convert service_id address to and from string", () => {
        let addr = createServiceAddress("service_id-1");
        let str = JSON.stringify(addr);
        let parsed = parseAddress(str);

        expect(parsed).to.deep.equal(addr)
    });

    it("should be able to convert peer address to and from string", () => {
        let pid = PeerId.createFromB58String("QmXduoWjhgMdx3rMZXR3fmkHKdUCeori9K1XkKpqeF5DrU");
        let addr = createPeerAddress(pid.toB58String());
        let str = JSON.stringify(addr);
        let parsed = parseAddress(str);

        expect(parsed).to.deep.equal(addr)
    });

    it("should be able to convert relay address to and from string", () => {
        let pid = PeerId.createFromB58String("QmXduoWjhgMdx3rMZXR3fmkHKdUCeori9K1XkKpqeF5DrU");
        let relayid = PeerId.createFromB58String("QmWySxQsFWPHdTLMqhJb4DYrTiFEge2tLe7FksRGHuPiTh");
        let addr = createRelayAddress(relayid.toB58String(), pid.toB58String());
        let str = JSON.stringify(addr);
        let parsed = parseAddress(str);

        expect(parsed).to.deep.equal(addr)
    });

    it("should be able to convert function call to and from string", () => {
        let pid = PeerId.createFromB58String("QmXduoWjhgMdx3rMZXR3fmkHKdUCeori9K1XkKpqeF5DrU");
        let relayid = PeerId.createFromB58String("QmWySxQsFWPHdTLMqhJb4DYrTiFEge2tLe7FksRGHuPiTh");
        let addr = createRelayAddress(relayid.toB58String(), pid.toB58String());

        let pid2 = PeerId.createFromB58String("QmXduoWjhgMdx3rMZXR3fmkHKdUCeori9K1XkKpqeF5DrU");
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

        let str = JSON.stringify(functionCall);

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

        let str2 = JSON.stringify(functionCallWithOptional);

        let parsed2 = parseFunctionCall(str2);

        expect(parsed2).to.deep.equal(functionCallWithOptional)

    });

    it("should correct calculate hash", async () => {
        const data = Buffer.from('hello world!');
        let hash = await calcHash(data);
        expect(hash).to.equal("QmTp2hEo8eXRp6wg7jXv1BLCMh5a4F3B7buAUZNZUu772j");
    });
});
