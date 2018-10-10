import "bootstrap/dist/css/bootstrap.min.css";
import * as fluence from "js-fluence-client";
import {Result} from "js-fluence-client";

class DbClient {

    session: fluence.Session;

    constructor(host: string, port: number) {
        this.session = fluence.createDefaultSession(host, port)
    }

    async submitQuery(queries: string[]): Promise<Promise<Result>[]> {
        return queries.map((q) => {
            console.log("query: " + q);
            let command = `do_query("${q}")`;
            let res = this.session.invokeRaw(command).result();
            res.then((r: Result) => {
                if (fluence.isValue(r)) {
                    let strResult = fluence.fromHex(r.hex());
                    console.log(`the result is:\n ${strResult}`);
                }
            });
            return res;
        });
    }
}

let client = new DbClient("localhost", 46157);

let resultField: HTMLTextAreaElement = window.document.getElementById("result") as HTMLTextAreaElement;
let inputField: HTMLInputElement = window.document.getElementById("query") as HTMLInputElement;

let btn = document.getElementById("submitQuery");

if (btn !== null) {
    btn.addEventListener("click", () => {
        if (inputField.value !== null) {
            submitQueries(inputField.value)
        }
    });
}

let newLine = String.fromCharCode(13, 10);
let sep = "**************************";

export function submitQueries(queries: string) {
    resultField.value = "";
    client.submitQuery(queries.split('\n')).then((results) => {

        results.forEach((pr) => {
            pr.then((r) => {
                if (fluence.isValue(r)) {
                    let strRes = r.asString().replace('\\n', newLine);
                    resultField.value += sep + newLine + strRes + newLine + sep;
                }
            });

        });

    })
}