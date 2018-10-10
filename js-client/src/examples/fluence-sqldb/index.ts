import "bootstrap/dist/css/bootstrap.min.css";
import * as fluence from "js-fluence-client";


class DbClient {

    session: fluence.Session;

    constructor(host: string, port: number) {
        this.session = fluence.createDefaultSession(host, port)
    }

    async submitQuery(query: string): Promise<fluence.Empty | fluence.Value> {
        let command = `do_query("${query}")`;
        let res = await this.session.invokeRaw(command).result();
        if (fluence.isValue(res)) {
            let strResult = fluence.fromHex(res.hex());
            console.log(`the result is:\n ${strResult}`);
        } else {
            console.log(`the result is empty`);
        }
        return res;
    }
}

let client = new DbClient("localhost", 46157);

let resultField: HTMLTextAreaElement = window.document.getElementById("result") as HTMLTextAreaElement;
let inputField: HTMLInputElement = window.document.getElementById("query") as HTMLInputElement;

let btn = document.getElementById("submitQuery");

if (btn !== null) {
    btn.addEventListener("click", () => {
        if (inputField !== null && inputField.value !== null) {
            submitQuery(inputField.value)
        }
    });
}

export function submitQuery(query: string) {
    client.submitQuery(query).then((res) => {

        if (fluence.isValue(res) && resultField !== null) {
            let newLine = String.fromCharCode(13, 10);
            let str = fluence.fromHex(res.hex());
            resultField.value = str.replace('\\n', newLine);
        }
    })
}