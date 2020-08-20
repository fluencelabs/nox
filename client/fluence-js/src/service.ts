import {FluenceClient} from "./fluenceClient";

export class Service {
    private readonly client: FluenceClient;
    private readonly serviceId: string;
    private readonly peerId: string;

    constructor(client: FluenceClient, peerId: string, serviceId: string) {
        this.client = client;
        this.serviceId = serviceId;
        this.peerId = peerId;
    }

    async call(moduleId: string, args: any, fname?: string): Promise<any> {
        return this.client.callService(this.peerId, this.serviceId, moduleId, args, fname);
    }
}