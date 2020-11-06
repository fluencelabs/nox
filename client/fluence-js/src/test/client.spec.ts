import {expect} from 'chai';

import 'mocha';
import {encode} from "bs58"
import Fluence from "../fluence";
import {certificateFromString, certificateToString, issue} from "../trust/certificate";
import {TrustGraph} from "../trust/trust_graph";
import {nodeRootCert} from "../trust/misc";
import {peerIdToSeed, seedToPeerId} from "../seed";
import {build} from "../particle";
import {Service} from "../service";
import {registerService} from "../globalState";
import {waitResult} from "../helpers/waitService";

describe("Typescript usage suite", () => {

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
    it.skip("test certs", async function () {
        this.timeout(15000);
        await testCerts();
    });

    it.skip("", async function () {
        let pid = await Fluence.generatePeerId()
        let cl = await Fluence.connect("/ip4/138.197.177.2/tcp/9001/ws/p2p/12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9", pid)

        let service = new Service("test")
        service.registerFunction("test", (args: any[]) => {
            console.log("called: " + args)
            return {}
        })
        registerService(service);

        let namedPromise = waitResult(30000)

        let script = `
(seq (
    (call ( "${pid.toB58String()}" ("test" "test") (a b c d) result))
    (call ( "${pid.toB58String()}" ("${namedPromise.name}" "") (d c b a) void[]))
))
`

        let data: Map<string, any> = new Map();
        data.set("a", "some a")
        data.set("b", "some b")
        data.set("c", "some c")
        data.set("d", "some d")

        let particle = await build(pid, script, data, 30000)

        await cl.sendParticle(particle)

        let res = await namedPromise.promise
        expect(res).to.be.equal(["some d", "some c", "some b", "some a"])
    })
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

    await cl1.disconnect();
    await cl2.disconnect();
}
