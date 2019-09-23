import {Result} from "./Result";

export enum ExecutorType {
    Promise = "promise",
    Subscription = "subscription"
}

/**
 * Structure, that could handle a result or an error in any time after creation.
 */
export abstract class Executor<T> {

    static isPromise<A>(executor: Executor<A>): executor is PromiseExecutor<A> {
        return executor.type === ExecutorType.Promise
    }

    abstract complete(result: T): void

    abstract fail(error: any): void

    abstract type: ExecutorType
}

/**
 * Executor based on promise. Terminates once.
 */
export class PromiseExecutor<T> extends Executor<T> {
    private resultCallback: (result: T) => void;
    private errorCallback: (error: any) => void;
    readonly promise: Promise<T>;
    readonly creationTime: number;
    private timeout: ReturnType<typeof setTimeout> | undefined;

    constructor(timeout: ReturnType<typeof setTimeout> | undefined) {
        super();
        this.promise = new Promise<T>((r, e) => { this.resultCallback = r; this.errorCallback = e; });
        this.timeout = timeout;
    }

    complete(result: T): void {
        this.resultCallback(result)
    }

    fail(error: any): void {
        this.errorCallback(error)
    }

    cancelTimeout() {
        if (this.timeout) {
            clearTimeout(this.timeout)
        }
    }

    type: ExecutorType = ExecutorType.Promise
}

/**
 * Executor based on callbacks
 */
export class SubscribtionExecutor extends Executor<Result> {
    readonly resultCallback: (result: Result) => void;
    readonly errorCallback: (error: any) => void;
    readonly subscription: string;

    constructor(subscription: string, resultCallback: (result: Result) => void, errorCallback: (error: any) => void) {
        super();
        this.subscription = subscription;
        this.resultCallback = resultCallback;
        this.errorCallback = errorCallback;
    }

    fail(error: any): void {
        this.errorCallback(error);
    }

    complete(result: Result): void {
        this.resultCallback(result);
    }

    type: ExecutorType = ExecutorType.Subscription
}
