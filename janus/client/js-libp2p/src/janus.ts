import * as PeerInfo from "peer-info";
import * as PeerId from "peer-id";
import {JanusClient} from "./janus_client";


export default class Janus {

    static async generatePeerId(): Promise<PeerId> {
        return await PeerId.create();
    }

    static async connect(hostPeerId: string, host: string, port: number, peerId: PeerId): Promise<JanusClient> {
        let peerInfo = await PeerInfo.create(peerId);

        let client = new JanusClient(peerInfo);

        await client.connect(hostPeerId, host, port);

        return client;
    }
}