import {FunctionCall} from "./function_call";

export class Subscriptions {
    private subscriptions: ((req: FunctionCall) => (boolean | undefined))[] = [];

    constructor() {}

    /**
     * Subscriptions will be applied to all peer and relay messages.
     * If subscription returns true, delete subscription.
     * @param f
     */
    subscribe(f: (call: FunctionCall) => (boolean | undefined)) {
        this.subscriptions.push(f);
    }

    /**
     * Apply call to all subscriptions and delete subscriptions that return `true`.
     * @param call
     */
    applyToSubscriptions(call: FunctionCall) {
        // if subscription return false - delete it from subscriptions
        this.subscriptions = this.subscriptions.filter(callback => !callback(call))
    }
}
