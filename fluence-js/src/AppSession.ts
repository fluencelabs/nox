import {WorkerSession} from "./fluence";
import {ResultPromise} from "./ResultAwait";
import {PrivateKey} from "./utils";

// All sessions with workers from an app
export class AppSession {
    private sessionId: string;
    private appId: string;
    private workerSessions: WorkerSession[];
    private readonly privateKey?: PrivateKey;
    private counter: number;
    private workerCounter: number;

    // selects next worker and calls `request` on that worker
    request(payload: string): ResultPromise {
        let nextWorker = this.workerCounter++ % this.workerSessions.length;
        let session = this.workerSessions[nextWorker].session;
        return session.request(payload, this.privateKey, this.counter++);
    }

    constructor(sessionId: string, appId: string, workerSessions: WorkerSession[], privateKey?: PrivateKey) {
        this.counter = 0;
        this.workerCounter = 0;
        this.sessionId = sessionId;
        this.appId = appId;
        this.workerSessions = workerSessions;
        this.privateKey = privateKey;
    }
}

