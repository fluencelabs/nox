import {Result} from "./Result";

export enum ExecutorType {
    Promise = "promise",
    Subscription = "subscription"
}

/**
 * Structure, that could handle a result or an error in any time after creation.
 */
export abstract class Executor<T> {
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
    private _promise: Promise<T>;

    constructor() {
        super();
        this._promise = new Promise<T>((r, e) => { this.resultResolver = r; this.errorResolver = e; });
    }

    promise(): Promise<T> {
        return this._promise
    }

    handleResult(result: T): void {
        this.resultResolver(result)
    }

    handleError(error: any): void {
        this.errorResolver(error)
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
