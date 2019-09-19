import {Result} from "./Result";

export abstract class Executor<T> {
    abstract handleResult(result: T): void

    abstract handleError(error: any): void

    abstract type: string
}

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

    type: string = "promise"
}

export class SubscribtionExecutor extends Executor<Result> {
    resultHandler: (result: Result) => void;
    errorHandler: (error: any) => void;

    constructor(resultHandler: (result: Result) => void, errorHandler: (error: any) => void) {
        super();
        this.resultHandler = resultHandler;
        this.errorHandler = errorHandler;
    }

    handleError(error: any): void {
        this.errorHandler(error);
    }

    handleResult(result: Result): void {
        this.resultHandler(result);
    }

    type: string = "subscription"
}
