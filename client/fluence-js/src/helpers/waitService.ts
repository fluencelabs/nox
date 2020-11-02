/**
 * Creates service that will wait for a response from external peers.
 */
import {genUUID} from "../particle";
import log from "loglevel";
import {Service} from "../service";
import {deleteService, registerService} from "../globalState";
import {delay} from "../utils";

interface NamedPromise<T> {
    promise: Promise<T>,
    name: string
}

/**
 * Generates a service and a name of a service.
 * Name should be used in a script.
 * Promise will wait a result from a script or will be resolved after `ttl` milliseconds.
 * @param ttl
 */
export function waitResult(ttl: number): NamedPromise<any[]> {
    return waitService(genUUID(), (args: any[]) => args, ttl)
}

export function waitService<T>(functionName: string, func: (args: any[]) => T, ttl: number): NamedPromise<T> {
    let serviceName = `${functionName}-${genUUID()}`;
    log.info(`Create waiting service '${serviceName}'`)
    let service = new Service(serviceName)
    registerService(service)

    let promise: Promise<T> = new Promise(function (resolve) {
        service.registerFunction("", (args: any[]) => {
            resolve(func(args))
            return {}
        })
    })

    let timeout = delay<T>(ttl, "Timeout on waiting " + serviceName)

    return {
        name: serviceName,
        promise: Promise.race([promise, timeout]).finally(() => {
            deleteService(serviceName)
        })
    }
}