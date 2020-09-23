import {Particle} from "./particle";
import {FluenceClient} from "./fluenceClient";


export class PreparedParticle {

    script: string;
    client: FluenceClient;

    constructor(script: string, client: FluenceClient) {
        this.script = script;
        this.client = client;
    }

    async send(data: object, ttl?: number): Promise<Particle> {
        return this.client.sendParticle(this.script, data, ttl)
    }
}

