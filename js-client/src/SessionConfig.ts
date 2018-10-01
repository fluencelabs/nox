export class SessionConfig {
    readonly requestsPerSec: number;
    readonly checkSessionTimeout: number;
    readonly requestTimeout: number;

    constructor(_requestsPerSec: number = 2, _checkSessionTimeout: number = 5, _requestTimeout: number = 60) {
        this.requestsPerSec = _requestsPerSec;
        this.checkSessionTimeout = _checkSessionTimeout;
        this.requestTimeout = _requestTimeout;
    }
}