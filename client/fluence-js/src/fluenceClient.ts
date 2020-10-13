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


import {build, genUUID, Particle} from "./particle";
import {StepperOutcome} from "./stepperOutcome";
import * as PeerId from "peer-id";
import Multiaddr from "multiaddr"
import {FluenceConnection} from "./fluenceConnection";
import {Subscriptions} from "./subscriptions";
import {
    addParticle,
    deleteService,
    getCurrentParticleId,
    popParticle,
    registerService,
    setCurrentParticleId
} from "./globalState";
import {instantiateStepper, Stepper} from "./stepper";
import log from "loglevel";
import {Service} from "./callService";
import {delay} from "./utils";
import {toByteArray} from "base64-js";

interface WaitingService<T> {
    promise: Promise<T>,
    name: string
}

export class FluenceClient {
    readonly selfPeerId: PeerId;
    readonly selfPeerIdStr: string;

    private nodePeerIdStr: string;
    private subscriptions = new Subscriptions();
    private stepper: Stepper = undefined;

    connection: FluenceConnection;

    constructor(selfPeerId: PeerId) {
        this.selfPeerId = selfPeerId;
        this.selfPeerIdStr = selfPeerId.toB58String();
    }

    /**
     * Pass a particle to a stepper and send a result to other services.
     */
    private async handleParticle(particle: Particle): Promise<void> {

        // if a current particle is processing, add new particle to the queue
        if (getCurrentParticleId() !== undefined) {
            addParticle(particle);
        } else {
            if (this.stepper === undefined) {
                throw new Error("Undefined. Stepper is not initialized. User 'Fluence.connect' to create a client.")
            }
            // start particle processing if queue is empty
            try {
                let stepperOutcomeStr = this.stepper(particle.init_peer_id, particle.script, JSON.stringify(particle.data))
                let stepperOutcome: StepperOutcome = JSON.parse(stepperOutcomeStr);

                log.info("inner stepper outcome:");
                log.info(stepperOutcome);

                // do nothing if there is no `next_peer_pks`
                if (stepperOutcome.next_peer_pks.length > 0) {
                    let newParticle: Particle = {...particle};
                    newParticle.data = JSON.parse(stepperOutcome.data);

                    await this.connection.sendParticle(newParticle).catch((reason) => {
                        console.error(`Error on sending particle with id ${particle.id}: ${reason}`)
                    });
                }
            } finally {
                // get last particle from the queue
                let nextParticle = popParticle();
                // start the processing of a new particle if it exists
                if (nextParticle) {
                    // update current particle
                    setCurrentParticleId(nextParticle.id);
                    this.handleParticle(nextParticle)
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
            let now = Date.now();
            let data = particle.data;
            let error: any = data["protocol!error"]
            if (error !== undefined) {
                log.error("error in external particle: ")
                log.error(error)
            } else {
                if (particle.timestamp + particle.ttl > now) {
                    log.info("handle external particle: ")
                    log.info(particle)
                    await _this.handleParticle(particle);
                } else {
                    console.log(`Particle expired. Now: ${now}, ttl: ${particle.ttl}, ts: ${particle.timestamp}`)
                }
            }
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

        this.stepper = await instantiateStepper(this.selfPeerId);

        let connection = new FluenceConnection(multiaddr, peerId, this.selfPeerId, this.handleExternalParticle());

        await connection.connect();

        this.connection = connection;
    }

    async sendParticle(particle: Particle): Promise<string> {
        await this.handleParticle(particle);
        this.subscriptions.subscribe(particle.id, particle.ttl);
        return particle.id
    }

    nodeIdentityCall(): string {
        return `(call ("${this.nodePeerIdStr}" ("identity" "") () void0))`
    }

    /**
     * Creates service that will wait for a response from external peers.
     */
    private waitService<T>(functionName: string, func: (args: any[]) => T): WaitingService<T> {
        let serviceName = `${functionName}-${genUUID()}`;
        log.info(`Create waiting service '${serviceName}'`)
        let service = new Service(serviceName)
        registerService(service)

        let promise: Promise<T> = new Promise(function(resolve, reject){
            service.registerFunction("", (args: any[]) => {
                resolve(func(args))
                return {}
            })
        })

        let timeout = delay<T>(10000, "Timeout on waiting " + serviceName)

        return {
            name: serviceName,
            promise: Promise.race([promise, timeout]).finally(() => {
                deleteService(serviceName)
            })
        }
    }

    /**
     * Send a script to add module to a relay. Waiting for a response from a relay.
     */
    async addModule(name: string, moduleBase64: string): Promise<void> {
        let config = {
            name: name,
            mem_pages_count: 100,
            logger_enabled: true,
            wasi: {
                envs: {},
                preopened_files: ["/tmp"],
                mapped_dirs: {},
            }
        }

        let waitingService = this.waitService("addModule", (args: any[]) => {})

        let script = `(seq (
            ${this.nodeIdentityCall()}
            (seq (           
                (call ("${this.nodePeerIdStr}" ("add_module" "") (module_bytes module_config) void2))
                (call ("${this.selfPeerIdStr}" ("${waitingService.name}" "") () void1))
            ))
        ))
        `

        let data = {
            module_bytes: Array.from(toByteArray(moduleBase64)),
            module_config: config
        }

        let particle = await build(this.selfPeerId, script, data, 10000)
        await this.sendParticle(particle);

        return waitingService.promise
    }

    /**
     * Send a script to add module to a relay. Waiting for a response from a relay.
     */
    async addBlueprint(name: string, dependencies: string[]): Promise<string> {
        let waitingService = this.waitService("addBlueprint", (args: any[]) => args[0] as string)

        let script = `(seq (
            ${this.nodeIdentityCall()}
            (seq (           
                (call ("${this.nodePeerIdStr}" ("add_blueprint" "") (blueprint) blueprint_id))
                (call ("${this.selfPeerIdStr}" ("${waitingService.name}" "") (blueprint_id) void1))
            ))
        ))
        `

        let data = {
            blueprint: { name: name, dependencies: dependencies }
        }

        let particle = await build(this.selfPeerId, script, data, 10000)
        await this.sendParticle(particle);

        return waitingService.promise
    }

    /**
     * Send a script to create a service to a relay. Waiting for a response from a relay.
     */
    async createService(blueprintId: string): Promise<string> {
        let waitingService = this.waitService("createService", (args: any[]) => args[0] as string)

        let script = `(seq (
            ${this.nodeIdentityCall()}
            (seq (           
                (call ("${this.nodePeerIdStr}" ("create" "") (blueprint_id) service_id))
                (call ("${this.selfPeerIdStr}" ("${waitingService.name}" "") (service_id) void1))
            ))
        ))
        `

        let data = {
            blueprint: { blueprint_id: blueprintId }
        }

        let particle = await build(this.selfPeerId, script, data, 10000)
        await this.sendParticle(particle);

        return waitingService.promise
    }
}
