import {WorkerSession} from "./fluence";
import {ResultPromise} from "./ResultAwait";
import {PrivateKey} from "./utils";
import {getWorkerStatus} from "fluence-monitoring"

// All sessions with workers from an app
export class AppSession {
    private sessionId: string;
    private appId: string;
    private workerSessions: WorkerSession[];
    private readonly privateKey?: PrivateKey;
    private counter: number;
    private workerCounter: number;

    constructor(sessionId: string, appId: string, workerSessions: WorkerSession[], privateKey?: PrivateKey) {
        if (workerSessions.length == 0) {
            console.error("Empty workerSession passed to AppSession constructor");
            throw new Error("Empty workerSession passed to AppSession constructor");
        }

        this.counter = 0;
        this.workerCounter = 0;
        this.sessionId = sessionId;
        this.appId = appId;
        this.workerSessions = workerSessions;
        this.privateKey = privateKey;
    }

    // selects next worker and calls `request` on that worker
    request(payload: string): ResultPromise {
        let nextWorker = this.workerCounter++ % this.workerSessions.length;
        let session = this.workerSessions[nextWorker].session;
        return session.request(payload, this.privateKey, this.counter++);
    }

    // gets info about all workers in cluster
    getWorkersStatus(): Promise<any[]> {
        return Promise.all(this.workerSessions.map((session) => {
            return getWorkerStatus(session.node.ip_addr, session.node.api_port.toString(), parseInt(this.appId));
        }));
    }
}

