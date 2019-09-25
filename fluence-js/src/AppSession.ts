import {WorkerSession} from "./fluence";
import {PrivateKey} from "./utils";
import {getWorkerStatus} from "./contract";
import {RequestState, RequestStatus, Session} from "./Session";
import {ErrorType, Result} from "./Result";

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

    private async performRequest<T>(call: (session: Session) => Promise<RequestState<T>>, retryCount: number = 0): Promise<T> {
        const { session } = this.getNextWorkerSession();

        const { status, result, error } = await call(session);

        if (status !== RequestStatus.OK) {
            if (status === RequestStatus.E_REQUEST && retryCount < this.workerSessions.length) {
                if (error && error.errorType == ErrorType.TendermintError) {
                    throw error;
                }
                session.ban();
                console.log(`Worker's session is banned until ${session.banTime()} milliseconds cause of: ${error}`);
                return this.performRequest(call, retryCount + 1);
            }

            throw error;
        }

        return result as T;
    }

    /**
     * Sends transaction with payload to the cluster.
     *
     * @param payload Either an argument for Wasm VM main handler or a command for the statemachine
     *
     * @returns response to a completed request
     */
    async request(payload: string): Promise<Result> {
        if (this.allSessionsBanned()) {
            return Promise.reject("All sessions are banned");
        }

        const currentCounter = this.counter++;

        const result = await this.performRequest((s: Session) => s.request(payload, this.privateKey, currentCounter));

        if (result.isDefined) return result.get;
        else return Promise.reject("Unexpected. The result is empty.")
    }

    /**
     * Sends a transaction with a payload to the cluster. Returns a request id of transaction.
     * Use this method if there is no reason to wait a response.
     * Use `query` with this request id to get a response.
     * Or use `request` to get response right after sending a transaction.
     *
     * @param payload Either an argument for Wasm VM main handler or a command for the statemachine
     */
    async requestAsync(payload: string): Promise<string> {
        if (this.allSessionsBanned()) {
            return Promise.reject("All sessions are banned");
        }

        const currentCounter = this.counter++;

        return this.performRequest((s: Session) => s.requestAsync(payload, this.privateKey, currentCounter));
    }

    /**
     * Returns a response by request_id.
     *
     * @param requestId id of request
     *
     * @returns the result if it is found, or undefined if result is pending or there is no result in the cluster
     */
    async query(requestId: string): Promise<Result | undefined> {
        const result = await this.performRequest((s: Session) => s.query(requestId));

        if (result.isDefined) {
            return result.get
        } else {
            return undefined
        }
    }

    // gets info about all workers in the cluster
    getWorkersStatus(): Promise<any[]> {
        return Promise.all(this.workerSessions.map((session) => {
            return getWorkerStatus(session.node.ip_addr, session.node.api_port.toString(), parseInt(this.appId));
        }));
    }

    allSessionsBanned(): Boolean {
        return this.workerSessions.find((s) => !s.session.isBanned()) === undefined;
    }
}

