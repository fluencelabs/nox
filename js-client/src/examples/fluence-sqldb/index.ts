import {Session} from "js-fluence-client/dist/Session";
import {Client} from "js-fluence-client/dist/Client";
import {TendermintClient} from "js-fluence-client/dist/TendermintClient";
import {Engine} from "js-fluence-client/dist/Engine";
import {Signer} from "js-fluence-client/dist/Signer";
import {Empty, isValue, Value} from "js-fluence-client/dist/Result";
import {fromHex} from "js-fluence-client/dist/utils";
import "bootstrap/dist/css/bootstrap.min.css";


class DbClient {

    session: Session;

    constructor(host: string, port: number) {
        let tm = new TendermintClient(host, port);

        let engine = new Engine(tm);

        // default signing key for now
        let signingKey = "TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxT3hAtSkX+Lrl4lN5OEsXjD7GGG7iEewSod472HudrkrA==";
        let signer = new Signer(signingKey);

        // `client002` is a default client for now
        let client = new Client("client002", signer);

        this.session = engine.genSession(client);
    }

    async submitQuery(query: string): Promise<Empty | Value> {
        let command = `do_query("${query}")`;
        let res = await this.session.invokeRaw(command).result();
        if (isValue(res)) {
            let strResult = fromHex(res.hex());
            console.log(`the result is:\n ${strResult}`);
        } else {
            console.log(`the result is empty`);
        }
        return res;
    }
}

let client = new DbClient("localhost", 46157);

let resultField = window.document.getElementById("result");
let inputField = window.document.getElementById("query");

export function submitQuery(query: string) {
    client.submitQuery(query).then((res) => {
        return res;
    })
}