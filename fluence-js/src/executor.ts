import {Result} from "./Result";

export enum ExecutorType {
    Promise = "promise",
    Subscription = "subscription"
}

/**
 * Structure, that could handle a result or an error in any time after creation.
 */
export abstract class Executor<T> {

    static isPromise<A>(executor: Executor<A>): boolean {
        return executor.isPromise()
    }

    isPromise(): boolean {
        return this.type == ExecutorType.Promise
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
    private timer: ReturnType<typeof setTimeout> | undefined;

    constructor(timer: ReturnType<typeof setTimeout> | undefined) {
        super();
        this.promise = new Promise<T>((r, e) => { this.resultCallback = r; this.errorCallback = e; });
        this.timer = timer;
    }

    complete(result: T): void {
        this.resultCallback(result)
        this.cancelTimeout()
    }

    fail(error: any): void {
        this.errorCallback(error)
        this.cancelTimeout()
    }

    cancelTimeout() {
        if (this.timer) {
            clearTimeout(this.timer)
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
