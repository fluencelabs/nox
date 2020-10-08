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


import {Particle} from "./particle";
import {StepperOutcome} from "./stepperOutcome";
import * as PeerId from "peer-id";
import Multiaddr from "multiaddr"
import {FluenceConnection} from "./fluenceConnection";
import {Subscriptions} from "./subscriptions";
import {addParticle, getCurrentParticleId, popParticle, setCurrentParticleId} from "./globalState";
import {instantiateStepper, Stepper} from "./stepper";
import log from "loglevel";

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

    handleParticle(particle: Particle): void {

        // if a current particle is processing, add new particle to the queue
        if (getCurrentParticleId()) {
            addParticle(particle);
        } else {
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

                    this.connection.sendParticle(newParticle).catch((reason) => {
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
     * Handle incoming call.
     * If FunctionCall returns - we should send it as a response.
     */
    handleExternalParticle(): (particle: Particle) => void {

        let _this = this;

        return (particle: Particle) => {
            let now = Date.now();
            if (particle.timestamp + particle.ttl > now) {
                _this.handleParticle(particle);
            } else {
                console.log(`Particle expired. Now: ${now}, ttl: ${particle.ttl}, ts: ${particle.timestamp}`)
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

    sendParticle(particle: Particle): string {
        this.handleParticle(particle);
        this.subscriptions.subscribe(particle.id, particle.ttl);
        return particle.id
    }
}
