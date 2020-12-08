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


import {build, Particle} from "./particle";
import {StepperOutcome} from "./stepperOutcome";
import * as PeerId from "peer-id";
import Multiaddr from "multiaddr"
import {FluenceConnection} from "./fluenceConnection";
import {Subscriptions} from "./subscriptions";
import {
    enqueueParticle,
    getCurrentParticleId,
    popParticle,
    setCurrentParticleId
} from "./globalState";
import {instantiateInterpreter, InterpreterInvoke} from "./stepper";
import log from "loglevel";
import {waitService} from "./helpers/waitService";
import {ModuleConfig} from "./moduleConfig";

const bs58 = require('bs58')

export class FluenceClient {
    readonly selfPeerId: PeerId;
    readonly selfPeerIdStr: string;

    private nodePeerIdStr: string;
    private subscriptions = new Subscriptions();
    private interpreter: InterpreterInvoke = undefined;

    connection: FluenceConnection;

    constructor(selfPeerId: PeerId) {
        this.selfPeerId = selfPeerId;
        this.selfPeerIdStr = selfPeerId.toB58String();
    }

    /**
     * Pass a particle to a interpreter and send a result to other services.
     */
    private async handleParticle(particle: Particle): Promise<void> {

        // if a current particle is processing, add new particle to the queue
        if (getCurrentParticleId() !== undefined && getCurrentParticleId() !== particle.id) {
            enqueueParticle(particle);
        } else {
            if (this.interpreter === undefined) {
                throw new Error("Undefined. Interpreter is not initialized. Use 'Fluence.connect' to create a client.")
            }
            // start particle processing if queue is empty
            try {
                setCurrentParticleId(particle.id)
                // check if a particle is relevant
                let now = Date.now();
                let actualTtl = particle.timestamp + particle.ttl - now;
                if (actualTtl <= 0) {
                    log.info(`Particle expired. Now: ${now}, ttl: ${particle.ttl}, ts: ${particle.timestamp}`)
                } else {
                    // if there is no subscription yet, previous data is empty
                    let prevData = [];
                    let prevParticle = this.subscriptions.get(particle.id);
                    if (prevParticle) {
                        prevData = prevParticle.data;
                        // update a particle in a subscription
                        this.subscriptions.update(particle)
                    } else {
                        // set a particle with actual ttl
                        this.subscriptions.subscribe(particle, actualTtl)
                    }
                    let stepperOutcomeStr = this.interpreter(particle.init_peer_id, particle.script, JSON.stringify(prevData), JSON.stringify(particle.data))
                    let stepperOutcome: StepperOutcome = JSON.parse(stepperOutcomeStr);

                    log.info("inner interpreter outcome:");
                    log.info(stepperOutcome);

                    // do nothing if there is no `next_peer_pks`
                    if (stepperOutcome.next_peer_pks.length > 0) {
                        let newParticle: Particle = {...particle};
                        newParticle.data = JSON.parse(stepperOutcome.call_path);

                        await this.connection.sendParticle(newParticle).catch((reason) => {
                            console.error(`Error on sending particle with id ${particle.id}: ${reason}`)
                        });
                    }
                }
            } finally {
                // get last particle from the queue
                let nextParticle = popParticle();
                // start the processing of a new particle if it exists
                if (nextParticle) {
                    // update current particle
                    setCurrentParticleId(nextParticle.id);
                    await this.handleParticle(nextParticle)
                } else {
                    // wait for a new call (do nothing) if there is no new particle in a queue
                    setCurrentParticleId(undefined);
                }
            }
        }
    }

    /**
     * Handle incoming particle from a relay.
     */
    private handleExternalParticle(): (particle: Particle) => Promise<void> {

        let _this = this;

        return async (particle: Particle) => {
            let data = particle.data;
            let error: any = data["protocol!error"]
            if (error !== undefined) {
                log.error("error in external particle: ")
                log.error(error)
            } else {
                log.info("handle external particle: ")
                log.info(particle)
                await _this.handleParticle(particle);
            }
        }
    }

    async disconnect(): Promise<void> {
        return this.connection.disconnect();
    }

    /**
     * Instantiate WebAssembly with AIR interpreter to execute AIR scripts
     */
    async instantiateInterpreter() {
        this.interpreter = await instantiateInterpreter(this.selfPeerId);
    }

    /**
     * Establish a connection to the node. If the connection is already established, disconnect and reregister all services in a new connection.
     *
     * @param multiaddr
     */
    async connect(multiaddr: string | Multiaddr) {
        multiaddr = Multiaddr(multiaddr);

        if (!this.interpreter) {
            throw Error("you must call 'instantiateInterpreter' before 'connect'")
        }

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

        let node = PeerId.createFromB58String(nodePeerId);
        let connection = new FluenceConnection(multiaddr, node, this.selfPeerId, this.handleExternalParticle());
        await connection.connect();

        this.connection = connection;
    }

    async sendParticle(particle: Particle): Promise<string> {
        await this.handleParticle(particle);
        return particle.id
    }

    nodeIdentityCall(): string {
        return `(call "${this.nodePeerIdStr}" ("op" "identity") [] void[])`
    }

    async requestResponse<T>(name: string, call: (nodeId: string) => string, returnValue: string, data: Map<string, any>, handleResponse: (args: any[]) => T, nodeId?: string, ttl?: number): Promise<T> {
        if (!ttl) {
            ttl = 10000
        }

        if (!nodeId) {
            nodeId = this.nodePeerIdStr
        }

        let serviceCall = call(nodeId)

        let namedPromise = waitService(name, handleResponse, ttl)

        let script = `(seq
            ${this.nodeIdentityCall()}
            (seq 
                (seq          
                    ${serviceCall}
                    ${this.nodeIdentityCall()}
                )
                (call "${this.selfPeerIdStr}" ("${namedPromise.name}" "") [${returnValue}] void[])
            )
        )
        `

        let particle = await build(this.selfPeerId, script, data, ttl)
        await this.sendParticle(particle);

        return namedPromise.promise
    }

    /**
     * Send a script to add module to a relay. Waiting for a response from a relay.
     */
    async addModule(name: string, moduleBase64: string, config?: ModuleConfig, nodeId?: string, ttl?: number): Promise<void> {
        if (!config) {
            config = {
                name: name,
                mem_pages_count: 100,
                logger_enabled: true,
                wasi: {
                    envs: {},
                    preopened_files: ["/tmp"],
                    mapped_dirs: {},
                }
            }
        }

        let data = new Map()
        data.set("module_bytes", moduleBase64)
        data.set("module_config", config)

        let call = (nodeId: string) => `(call "${nodeId}" ("dist" "add_module") [module_bytes module_config] void[])`

        return this.requestResponse("addModule", call, "", data, () => {}, nodeId, ttl)
    }

    /**
     * Send a script to add module to a relay. Waiting for a response from a relay.
     */
    async addBlueprint(name: string, dependencies: string[], blueprintId?: string, nodeId?: string, ttl?: number): Promise<string> {
        let returnValue = "blueprint_id";
        let call = (nodeId: string) => `(call "${nodeId}" ("dist" "add_blueprint") [blueprint] ${returnValue})`

        let data = new Map()
        data.set("blueprint", { name: name, dependencies: dependencies, id: blueprintId })

        return this.requestResponse("addBlueprint", call, returnValue, data, (args: any[]) => args[0] as string, nodeId, ttl)
    }

    /**
     * Send a script to create a service to a relay. Waiting for a response from a relay.
     */
    async createService(blueprintId: string, nodeId?: string, ttl?: number): Promise<string> {
        let returnValue = "service_id";
        let call = (nodeId: string) => `(call "${nodeId}" ("srv" "create") [blueprint_id] ${returnValue})`

        let data = new Map()
        data.set("blueprint_id", blueprintId)

        return this.requestResponse("createService", call, returnValue, data, (args: any[]) => args[0] as string, nodeId, ttl)
    }

    /**
     * Get all available modules hosted on a connected relay.
     */
    async getAvailableModules(nodeId?: string, ttl?: number): Promise<string[]> {
        let returnValue = "modules";
        let call = (nodeId: string) => `(call "${nodeId}" ("dist" "get_modules") [] ${returnValue})`

        return this.requestResponse("getAvailableModules", call, returnValue, new Map(), (args: any[]) => args[0] as string[], nodeId, ttl)
    }

    /**
     * Get all available blueprints hosted on a connected relay.
     */
    async getBlueprints(nodeId: string, ttl?: number): Promise<string[]> {
        let returnValue = "blueprints";
        let call = (nodeId: string) => `(call "${nodeId}" ("dist" "get_blueprints") [] ${returnValue})`

        return this.requestResponse("getBlueprints", call, returnValue, new Map(), (args: any[]) => args[0] as string[], nodeId, ttl)
    }

    /**
     * Add a provider to DHT network to neighborhood around a key.
     */
    async addProvider(key: Buffer, providerPeer: string, providerServiceId?: string, nodeId?: string, ttl?: number): Promise<void> {
        let call = (nodeId: string) => `(call "${nodeId}" ("dht" "add_provider") [key provider] void[])`

        key = bs58.encode(key)

        let provider = {
            peer: providerPeer,
            service_id: providerServiceId
        }

        let data = new Map()
        data.set("key", key)
        data.set("provider", provider)

        return this.requestResponse("addProvider", call, "", data, () => {}, nodeId, ttl)
    }

    /**
     * Get a provider from DHT network from neighborhood around a key..
     */
    async getProviders(key: Buffer, nodeId?: string, ttl?: number): Promise<any> {
        key = bs58.encode(key)

        let returnValue = "providers"
        let call = (nodeId: string) => `(call "${nodeId}" ("dht" "get_providers") [key] providers[])`

        let data = new Map()
        data.set("key", key)

        return this.requestResponse("getProviders", call, returnValue, data, (args) => args[0], nodeId, ttl)
    }

    /**
     * Get relays neighborhood
     */
    async neighborhood(node: string, ttl?: number): Promise<string[]> {
        let returnValue = "neighborhood"
        let call = (nodeId: string) => `(call "${nodeId}" ("dht" "neighborhood") [node] ${returnValue})`

        let data = new Map()
        data.set("node", node)

        return this.requestResponse("neighborhood", call, returnValue, data, (args) => args[0] as string[], node, ttl)
    }

    /**
     * Call relays 'identity' method. It should return passed 'fields'
     */
    async relayIdentity(fields: string[], data: Map<string, any>, nodeId?: string, ttl?: number): Promise<any> {
        let returnValue = "id";
        let call = (nodeId: string) => `(call "${nodeId}" ("op" "identity") [${fields.join(" ")}] ${returnValue})`

        return this.requestResponse("getIdentity", call, returnValue, data, (args: any[]) => args[0], nodeId, ttl)
    }
}
