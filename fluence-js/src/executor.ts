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
    private resultHandler: (result: T) => void;
    private errorHandler: (error: any) => void;
    readonly promise: Promise<T>;
    private timeout: ReturnType<typeof setTimeout> | undefined;

    static withTimeout<T>(timeout: number, onTimeout: () => void) {
        return new PromiseExecutor<T>(timeout, onTimeout)
    }

    static create<T>() {
        return new PromiseExecutor<T>(undefined, undefined)
    }

    private constructor(timeout: number | undefined, onTimeout: (() => void) | undefined) {
        super();

        if (timeout && onTimeout) {
            this.timeout = setTimeout(() => {
                onTimeout()
            }, timeout);

            this.promise = new Promise<T>((r, e) => { this.resultHandler = r; this.errorHandler = e; })
                .finally(() => {
                    if (timeout) { clearTimeout(timeout) }
                });
        }
    }

    handleResult(result: T): void {
        this.resultHandler(result)
    }

    handleError(error: any): void {
        this.errorHandler(error)
    }

    type: ExecutorType = ExecutorType.Promise
}

/**
 * Executor based on callbacks. Could handle multiple results.
 */
export class SubscriptionExecutor extends Executor<Result> {
    readonly resultHandler: (result: Result) => void;
    readonly errorHandler: (error: any) => void;
    readonly subscription: string;

    constructor(subscription: string, resultCallback: (result: Result) => void, errorCallback: (error: any) => void) {
        super();
        this.subscription = subscription;
        this.resultHandler = resultCallback;
        this.errorHandler = errorCallback;
    }

    handleError(error: any): void {
        this.errorHandler(error);
    }

    handleResult(result: Result): void {
        this.resultHandler(result);
    }

    type: ExecutorType = ExecutorType.Subscription
}
