import {Result} from "./Result";

export enum ExecutorType {
    Promise = "promise",
    Subscription = "subscription"
}

/**
 * Structure, that could handle a result or an error in any time after creation.
 */
export abstract class Executor<T> {
    /**
     * Submits a result to executor.
     */
    abstract success(result: T): void

    /**
     * Submits an error to executor.
     * @param error
     */
    abstract fail(error: any): void

    abstract type: ExecutorType
}

/**
 * Executor based on promise. Terminates once.
 */
export class PromiseExecutor<T> extends Executor<T> {
    private resultHandler: (result: T) => void;
    private errorHandler: (error: any) => void;
    readonly promise: Promise<T>;
    private timerHandle: ReturnType<typeof setTimeout> | undefined;
    private _failed: boolean = false;

    constructor(onComplete: () => void = () => {}, onTimeout: () => void = () => {}, timeout: number | undefined = undefined) {
        super();

        this.promise = new Promise<T>((r, e) => {
            this.resultHandler = r;
            this.errorHandler = e;
        });

        if (timeout) {
            this.timerHandle = setTimeout(() => {
                onTimeout()
            }, timeout);
        }

        this.promise.finally(() => {
            onComplete();
            if (this.timerHandle) { clearTimeout(this.timerHandle) }
        });
    }

    success(result: T): void {
        this.resultHandler(result);
    }

    fail(error: any): void {
        this._failed = true;
        this.errorHandler(error)
    }

    isFailed(): boolean {
        return this._failed
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

    fail(error: any): void {
        this.errorHandler(error);
    }

    success(result: Result): void {
        this.resultHandler(result);
    }

    type: ExecutorType = ExecutorType.Subscription
}
