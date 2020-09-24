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


import {genUUID, Particle, signParticle} from "./particle";
import * as PeerId from "peer-id";
import Multiaddr from "multiaddr"
import {FluenceConnection} from "./fluenceConnection";
import {Subscriptions} from "./subscriptions";

export class FluenceClient {
    readonly selfPeerId: PeerId;
    readonly selfPeerIdStr: string;
    readonly defaultTtl: number;
    private nodePeerIdStr: string;
    private subscriptions = new Subscriptions();

    connection: FluenceConnection;

    constructor(selfPeerId: PeerId, defaultTtl?: number) {
        this.selfPeerId = selfPeerId;
        this.selfPeerIdStr = selfPeerId.toB58String();
        this.defaultTtl = defaultTtl ?? 7000;
    }

    /**
     * Waits a response that match the predicate.
     *
     * @param id
     * @param ttl
     */
    waitResponse(id: string, ttl: number): Promise<Particle> {
        return new Promise((resolve, reject) => {
            // subscribe for responses, to handle response
            // TODO if there's no conn, reject
            this.subscriptions.subscribe(id, (particle: Particle) => {
                resolve(particle);
            }, ttl);
        })
    }

    /**
     * Handle incoming call.
     * If FunctionCall returns - we should send it as a response.
     */
    handleParticle(): (particle: Particle) => void {

        let _this = this;

        return (particle: Particle) => {
            // call all subscriptions for a new call
            _this.subscriptions.applyToSubscriptions(particle);
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
        let connection = new FluenceConnection(multiaddr, peerId, this.selfPeerId, this.handleParticle());

        await connection.connect();

        this.connection = connection;
    }

    async sendParticle(script: string, data: object, ttl?: number): Promise<Particle> {
        let id = genUUID();
        let currentTime = (new Date()).getTime()
        let bigCurrentTime = BigInt(currentTime);

        ttl = ttl ?? this.defaultTtl
        let signature = await signParticle(this.selfPeerId, id, bigCurrentTime, ttl, script);
        let particle: Particle = {
            id: id,
            init_peer_id: this.selfPeerIdStr,
            timestamp: currentTime,
            ttl: ttl,
            script: script,
            signature: signature,
            data: data,
            action: "Particle"
        }
        await this.connection.sendParticle(particle);
        return this.waitResponse(id, ttl);
    }
}
