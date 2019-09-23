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

    abstract handleResult(result: T): void

    abstract handleError(error: any): void

    abstract type: ExecutorType
}

/**
 * Executor based on promise. Terminates once.
 */
export class PromiseExecutor<T> extends Executor<T> {
    private resultResolver: (result: T) => void;
    private errorResolver: (error: any) => void;
    readonly promise: Promise<T>;
    readonly creationTime: number;
    private timeout: ReturnType<typeof setTimeout> | undefined;

    constructor(timeout: ReturnType<typeof setTimeout> | undefined) {
        super();
        this.promise = new Promise<T>((r, e) => { this.resultResolver = r; this.errorResolver = e; });
        this.timeout = timeout;
    }

    handleResult(result: T): void {
        this.resultResolver(result)
    }

    handleError(error: any): void {
        this.errorResolver(error)
    }

    cancelTimeout() {
        if (this.timeout) {
            clearTimeout(this.timeout)
        }
    }

    type: ExecutorType = ExecutorType.Promise
}

/**
 * Executor based on callbacks,
 */
export class SubscribtionExecutor extends Executor<Result> {
    readonly resultHandler: (result: Result) => void;
    readonly errorHandler: (error: any) => void;
    readonly subscription: string;

    constructor(subscription: string, resultHandler: (result: Result) => void, errorHandler: (error: any) => void) {
        super();
        this.subscription = subscription;
        this.resultHandler = resultHandler;
        this.errorHandler = errorHandler;
    }

    handleError(error: any): void {
        this.errorHandler(error);
    }

    handleResult(result: Result): void {
        this.resultHandler(result);
    }

    type: ExecutorType = ExecutorType.Subscription
}
