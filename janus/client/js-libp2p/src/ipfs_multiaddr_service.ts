import {JanusConnection} from "./janus_connection";
import {FunctionCall, genUUID, makeFunctionCall} from "./function_call";

export class IpfsMultiaddrService {

    connection: JanusConnection;
    multiaddr: string;

    constructor(connection: JanusConnection, multiaddr: string) {
        this.connection = connection;
        this.multiaddr = multiaddr;
    }

    service(): (call: FunctionCall) => Promise<void> {

        let _this = this;

        return async (call: FunctionCall) => {
            let uuid = genUUID();

            console.log("MULTIADDR SERVICE: call received:");
            console.log(call);

            if (call.reply_to) {
                let response: FunctionCall = makeFunctionCall(uuid, call.reply_to, { multiaddr: _this.multiaddr, msg_id: call.arguments.msg_id }, undefined, "IPFS.multiaddr");

                console.log("MULTIADDR SERVICE: send response:");
                console.log(response);
                console.log("");

                _this.connection.sendFunctionCall(response);
            } else {
                console.log(`IpfsMultiaddrService: no 'reply_to' in call: ${JSON.stringify(call)}`)
            }
        };


    }
}
