import {JanusConnection} from "./janus_connection";
import {FunctionCall, genUUID, makeFunctionCall} from "./function_call";
import {calcHash, ipfsAdd} from "./ipfs_service";
import {createServiceAddress} from "./address";

export class IpfsShowService {

    connection: JanusConnection;
    file: Uint8Array;
    private hash: string;
    multiaddrService: string;
    _this = this;

    /**
     *
     * @param connection janus connection
     * @param file bytes in base64
     * @param multiaddrService service name to get IPFS node multiaddr
     */
    constructor(connection: JanusConnection, file: Uint8Array, multiaddrService: string) {
        this.connection = connection;
        this.file = file;
        this.multiaddrService = multiaddrService;
    }

    async initHash(): Promise<void> {
        if (!this.hash) {
            let buf = Buffer.from(this.file);
            this.hash = await calcHash(buf);
        }
    }

    async getHash(): Promise<string> {
        await this.initHash();
        return this.hash;
    }

    async getName(): Promise<string> {
        if (this.hash) {
            console.log("without calculation");
            return "IPFS.get_" + this.hash;
        } else {
            console.log("with calculation");
            await this.initHash();
            return "IPFS.get_" + this.hash;
        }
    }

    service(): (call: FunctionCall) => Promise<void> {

        console.log("HASH: " + this.hash);

        let thisService = this;

        return async (call: FunctionCall) => {

            if (!thisService.hash) {
                throw "IPFSSHOWSERVCE: calculate hash with `initHash` method before use"
            }

            let uuid = genUUID();

            let msgIdIpfsAddr = uuid + thisService.hash;

            let ipfsServiceResponse: Promise<string> = new Promise((resolve, reject) => {
                thisService.connection.subscribe((call: FunctionCall) => {
                    if (call.arguments.msg_id && call.arguments.msg_id === msgIdIpfsAddr) {
                        if (call.arguments.multiaddr) {
                            resolve(call.arguments.multiaddr as string);
                            return true;
                        } else {
                            reject("IPFSSHOWSERVICE: `msgId` equals but there is no `multiaddr`, delete subscription but can't do anything further");
                            return true;
                        }
                    }
                })
            });


            let ipfsAddrService = createServiceAddress(thisService.multiaddrService);
            let ipfsAddrCall: FunctionCall = makeFunctionCall(uuid, ipfsAddrService, {msg_id: msgIdIpfsAddr}, thisService.connection.replyToAddress, thisService.multiaddrService);

            console.log("IPFSSHOWSERVICE: send call to multiaddr:");
            console.log(ipfsAddrCall);
            console.log("");

            await thisService.connection.sendFunctionCall(ipfsAddrCall);

            let multiaddr = await ipfsServiceResponse;

            console.log("IPFSSHOWSERVICE: MULTIADDR RECEIVED: " + multiaddr);


            await ipfsAdd(multiaddr, thisService.file);
            console.log("IPFSSHOWSERVICE: FILE UPLOADED");

            let serviceName = await thisService.getName();
            let response = makeFunctionCall(genUUID(), call.reply_to, { multiaddr: multiaddr, msg_id: call.arguments.msg_id}, undefined, "response to " + serviceName);

            await thisService.connection.sendFunctionCall(response);

            /*
            to check this part:
            - browser1: create this service
            - browser2: create service Multiaddr
            - browser3: create peer and call first service
            - browser1: check that correct multiaddr returns
             */
        }
    }
}
