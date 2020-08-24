/*
 * Copyright 2020 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

import {
    Address,
    createPeerAddress,
    createRelayAddress,
    createProviderAddress,
    ProtocolType,
    addressToString
} from "./address";
import {callToString, FunctionCall, genUUID, makeFunctionCall,} from "./functionCall";
import * as PeerId from "peer-id";
import {LocalServices} from "./localServices";
import Multiaddr from "multiaddr"
import {Subscriptions} from "./subscriptions";
import * as PeerInfo from "peer-info";
import {FluenceConnection} from "./fluenceConnection";
import {checkInterface, Interface} from "./Interface";
import {Service} from "./service";
import {Blueprint, checkBlueprint} from "./blueprint";

/**
 * @param target receiver
 * @param args message in the call
 * @param moduleId module name
 * @param fname function name
 * @param name common field for debug purposes
 * @param msgId hash that will be added to replyTo address
 */
interface Call {
    target: Address,
    args: any,
    moduleId?: string,
    fname?: string,
    msgId?: string,
    name?: string
}

export class FluenceClient {
    readonly selfPeerInfo: PeerInfo;
    readonly selfPeerIdStr: string;
    private nodePeerIdStr: string;

    private connection: FluenceConnection;

    private services: LocalServices = new LocalServices();

    private subscriptions: Subscriptions = new Subscriptions();

    constructor(selfPeerInfo: PeerInfo) {
        this.selfPeerInfo = selfPeerInfo;
        this.selfPeerIdStr = selfPeerInfo.id.toB58String();
    }

    /**
     * Makes call with response from function. Without reply_to field.
     */
    private responseCall(target: Address, args: any): FunctionCall {
        return makeFunctionCall(genUUID(), target, this.connection.sender, args, undefined, "response");
    }

    /**
     * Waits a response that match the predicate.
     *
     * @param predicate will be applied to each incoming call until it matches
     * @param ignoreErrors ignore an errors, wait for success response
     */
    waitResponse(predicate: (args: any, target: Address, replyTo: Address) => (boolean | undefined), ignoreErrors: boolean): Promise<any> {
        return new Promise((resolve, reject) => {
            // subscribe for responses, to handle response
            // TODO if there's no conn, reject
            this.subscribe((args: any, target: Address, replyTo: Address) => {
                if (predicate(args, target, replyTo)) {
                    if (args.reason) {
                        if (ignoreErrors) {
                            return false;
                        } else {
                            reject(new Error(args.reason));
                        }
                    } else {
                        resolve(args);
                    }
                    return true;
                }
                return false;
            });
        });
    }

    private getPredicate(msgId: string): (args: any, target: Address) => (boolean | undefined) {
        return (args: any, target: Address) => target.hash && target.hash === msgId;
    }

    /**
     * Send call and forget.
     *
     */
    async sendCall(call: Call) {
        if (this.connection && this.connection.isConnected()) {
            await this.connection.sendFunctionCall(call.target, call.args, call.moduleId, call.fname, call.msgId, call.name);
        } else {
            throw Error("client is not connected")
        }
    }

    /**
     * Send call to the provider and wait a response matches predicate.
     *
     * @param provider published name in dht
     * @param args message to the service
     * @param moduleId module name
     * @param fname function name
     * @param name debug info
     */
    async callProvider(provider: string, args: any, moduleId?: string, fname?: string, name?: string): Promise<any> {
        let msgId = genUUID();
        let predicate = this.getPredicate(msgId);
        let address = createProviderAddress(provider);
        await this.sendCall({target: address, args: args, moduleId: moduleId, fname: fname, msgId: msgId, name: name});
        return await this.waitResponse(predicate, true);
    }

    /**
     * Send a call to the local service and wait a response matches predicate on a peer the client connected with.
     *
     * @param moduleId
     * @param addr node address
     * @param args message to the service
     * @param fname function name
     * @param name debug info
     */
    async callPeer(moduleId: string, args: any, fname?: string, addr?: string, name?: string): Promise<any> {
        let msgId = genUUID();
        let predicate = this.getPredicate(msgId);

        let address;
        if (addr) {
            address = createPeerAddress(addr);
        } else {
            address = createPeerAddress(this.nodePeerIdStr);
        }

        await this.sendCall({target: address, args: args, moduleId: moduleId, fname: fname, msgId: msgId, name: name})

        return await this.waitResponse(predicate, false);
    }

    async callService(peerId: string, serviceId: string, moduleId: string, args: any, fname?: string): Promise<any> {
        let target = createPeerAddress(peerId, serviceId);
        let msgId = genUUID();
        let predicate = this.getPredicate(msgId);

        await this.sendCall({target: target, args: args, moduleId: moduleId, fname: fname, msgId: msgId});
        return await this.waitResponse(predicate, false);
    }

    getService(peerId: string, serviceId: string): Service {
        return new Service(this, peerId, serviceId);
    }

    /**
     * Handle incoming call.
     * If FunctionCall returns - we should send it as a response.
     */
    handleCall(): (call: FunctionCall) => FunctionCall | undefined {

        let _this = this;

        return (call: FunctionCall) => {
            console.log("FunctionCall received:");

            // if other side return an error - handle it
            // TODO do it in the protocol
            /*if (call.arguments.error) {
                this.handleError(call);
            } else {

            }*/

            let target = call.target;

            // the tail of addresses should be you or your service
            let lastProtocol = target.protocols[target.protocols.length - 1];

            // call all subscriptions for a new call
            _this.subscriptions.applyToSubscriptions(call);

            switch (lastProtocol.protocol) {
                case ProtocolType.Providers:


                    return undefined;
                case ProtocolType.Client:
                    if (lastProtocol.value === _this.selfPeerIdStr) {
                        console.log(`relay call:`);
                        console.log(JSON.stringify(call, undefined, 2));
                        if (call.module) {
                            try {
                                // call of the service, service should handle response sending, error handling, requests to other services
                                let applied = _this.services.applyToService(call);

                                // if the request hasn't been applied, there is no such service. Return an error.
                                if (!applied) {
                                    console.log(`there is no service ${lastProtocol.value}`);
                                    return this.responseCall(call.reply_to, {
                                        reason: `there is no such service`,
                                        msg: call
                                    });
                                }
                            } catch (e) {
                                // if service throw an error, return it to the sender
                                return this.responseCall(call.reply_to, {
                                    reason: `error on execution: ${e}`,
                                    msg: call
                                });
                            }
                        }
                    } else {
                        console.warn(`this relay call is not for me: ${callToString(call)}`);
                        return this.responseCall(call.reply_to, {
                            reason: `this relay call is not for me`,
                            msg: call
                        });
                    }
                    return undefined;
                case ProtocolType.Peer:
                    if (lastProtocol.value === this.selfPeerIdStr) {
                        console.log(`peer call: ${call}`);
                    } else {
                        console.warn(`this peer call is not for me: ${callToString(call)}`);
                        return this.responseCall(call.reply_to, {
                            reason: `this relay call is not for me`,
                            msg: call
                        });
                    }
                    return undefined;
            }
        }
    }


    /**
     * Become a name provider. Other network members could find and call one of the providers of this name by this name.
     */
    async provideName(name: string, fn: (req: FunctionCall) => void) {
        let replyTo = this.connection.sender;
        await this.callPeer("provide", {name: name, address: addressToString(replyTo)})

        this.services.addService(name, fn);
    }

    /**
     * Sends a call to create a service on remote node.
     */
    async createService(peerId: string, blueprint: string): Promise<string> {
        let resp = await this.callPeer("create", {blueprint_id: blueprint}, undefined, peerId);

        if (resp && resp.service_id) {
            return resp.service_id
        } else {
            console.error("Unknown response type on `createService`: ", resp)
            throw new Error("Unknown response type on `createService`");
        }
    }

    async addBlueprint(peerId: string, name: string, dependencies: string[]): Promise<string> {

        let id = genUUID();
        let blueprint = {
            name: name,
            id: id,
            dependencies: dependencies
        };
        let msg_id = genUUID();

        await this.callPeer("add_blueprint", {msg_id, blueprint}, undefined, peerId);

        return id;
    }

    async getInterface(serviceId: string, peerId?: string): Promise<Interface> {
        let resp;
        resp = await this.callPeer("get_interface", {service_id: serviceId}, undefined, peerId)
        let i = resp.interface;

        if (checkInterface(i)) {
            return i;
        } else {
            throw new Error("Unexpected");
        }
    }

    async getAvailableBlueprints(peerId?: string): Promise<Blueprint[]> {
        let resp = await this.callPeer("get_available_blueprints", {}, undefined);

        let blueprints = resp.available_blueprints;

        if (blueprints && blueprints instanceof Array) {
            return blueprints.map((b: any) => {
                if (checkBlueprint(b)) {
                    return b;
                } else {
                    throw new Error("Unexpected");
                }
            });
        } else {
            throw new Error("Unexpected. 'get_active_interfaces' should return an array of interfaces.");
        }
    }

    async getActiveInterfaces(peerId?: string): Promise<Interface[]> {
        let resp = await this.callPeer("get_active_interfaces", {}, undefined);

        let interfaces = resp.active_interfaces;
        if (interfaces && interfaces instanceof Array) {
            return interfaces.map((i: any) => {
                if (checkInterface(i)) {
                    return i;
                } else {
                    throw new Error("Unexpected");
                }
            });
        } else {
            throw new Error("Unexpected. 'get_active_interfaces' should return an array pf interfaces.");
        }
    }

    async getAvailableModules(peerId?: string): Promise<string[]> {
        let resp = await this.callPeer("get_available_modules", {}, undefined, peerId);
        return resp.available_modules;
    }

    /**
     * Add new WASM module to the node.
     *
     * @param bytes WASM in base64
     * @param name WASM identificator
     * @param mem_pages_count memory amount for WASM
     * @param envs environment variables
     * @param mapped_dirs links to directories
     * @param preopened_files files and directories that will be used in WASM
     * @param peerId the node to add module
     */
    async addModule(bytes: string, name: string, mem_pages_count: number, envs: string[], mapped_dirs: any, preopened_files: string[], peerId?: string): Promise<any> {
        let config: any = {
            logger_enabled: true,
            mem_pages_count: mem_pages_count,
            name: name,
            wasi: {
                envs: envs,
                mapped_dirs: mapped_dirs,
                preopened_files: preopened_files
            }
        }
        let resp = await this.callPeer("add_module", {bytes: bytes, config: config}, undefined, peerId);

        return resp.available_modules;
    }

    // subscribe new hook for every incoming call, to handle in-service responses and other different cases
    // the hook will be deleted if it will return `true`
    subscribe(predicate: (args: any, target: Address, replyTo: Address) => (boolean | undefined)) {
        this.subscriptions.subscribe(predicate)
    }


    /**
     * Sends a call to unregister the service.
     */
    async unregisterService(moduleId: string) {
        if (this.services.deleteService(moduleId)) {
            console.warn("unregister is not implemented yet (service: ${serviceId}")
            // TODO unregister in fluence network when it will be supported
            // let regMsg = makeRegisterMessage(serviceId, PeerId.createFromB58String(this.nodePeerId));
            // await this.sendFunctionCall(regMsg);
        }
    }

    async disconnect(): Promise<void> {
        return this.connection.disconnect();
    }

    /**
     * Establish a connection to the node. If the connection is already established, disconnect and reregister all services in a new connection.
     *
     * @param multiaddr
     */
    async connect(multiaddr: string | Multiaddr): Promise<void> {

        multiaddr = Multiaddr(multiaddr);

        let nodePeerId = multiaddr.getPeerId();
        this.nodePeerIdStr = nodePeerId;

        if (!nodePeerId) {
            throw Error("'multiaddr' did not contain a valid peer id")
        }

        let firstConnection: boolean = true;
        if (this.connection) {
            firstConnection = false;
            await this.connection.disconnect();
        }

        let peerId = PeerId.createFromB58String(nodePeerId);
        let relayAddress = await createRelayAddress(nodePeerId, this.selfPeerInfo.id, true);
        let connection = new FluenceConnection(multiaddr, peerId, this.selfPeerInfo, relayAddress, this.handleCall());

        await connection.connect();

        this.connection = connection;

        // if the client already had a connection, it will reregister all services after establishing a new connection.
        if (!firstConnection) {
            for (let service of this.services.getAllServices().keys()) {
                await this.connection.provideName(service);
            }

        }

    }
}
