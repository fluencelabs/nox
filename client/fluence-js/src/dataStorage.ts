import {getCurrentParticleId, registerService} from "./globalState";
import {ServiceMultiple} from "./service";
import log from "loglevel";

let storage: Map<string, Map<string, any>> = new Map();

export function addData(particleId: string, data: Map<string, any>, ttl: number) {
    storage.set(particleId, data)
    setTimeout(() => {
        log.debug(`data for ${particleId} is deleted`)
        storage.delete(particleId)
    }, ttl)
}

export const storageService = new ServiceMultiple("")
storageService.registerFunction("load", (args: any[]) => {
    let current = getCurrentParticleId();

    let data = storage.get(current)

    if (data) {
        return data.get(args[0])
    } else {
        return {}
    }
})
registerService(storageService);
