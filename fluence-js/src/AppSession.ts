import {WorkerSession} from "./fluence";
import {ResultPromise} from "./ResultAwait";
import {PrivateKey} from "./utils";
import {getWorkerStatus} from "fluence-monitoring"
import {RequestStatus} from "./Session";

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

    private getNextWorkerSession(): WorkerSession {
        let workerSession;
        let counter = 0;
        do {
            if (counter++ >= this.workerSessions.length) {
                throw new Error('All sessions was banned, no free connection to use.');
            }

            const nextWorker = this.workerCounter++ % this.workerSessions.length;
            workerSession = this.workerSessions[nextWorker];
        } while(workerSession.session.isBanned());

        return workerSession;
    }

    // selects next worker and calls `request` on that worker
    async request(payload: string): Promise<ResultPromise> {
        const currentCounter = this.counter++;
        const performRequest = async (retryCount: number = 0): Promise<ResultPromise> => {
            const { session } = this.getNextWorkerSession();

            const { status, result, error } = await session.request(payload, this.privateKey, currentCounter);

            if (status !== RequestStatus.OK) {
                if (status === RequestStatus.E_REQUEST && retryCount < this.workerSessions.length) {
                    session.ban();
                    return performRequest(retryCount + 1);
                }

                throw error;
            }

            return result as ResultPromise;
        };

        return performRequest();
    }

    // gets info about all workers in the cluster
    getWorkersStatus(): Promise<any[]> {
        return Promise.all(this.workerSessions.map((session) => {
            return getWorkerStatus(session.node.ip_addr, session.node.api_port.toString(), parseInt(this.appId));
        }));
    }
}

