/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

import {Network} from "../types/web3-contracts/Network";

/**
 * An app is a WASM file. It can be deployed on a real-time cluster and run.
 * In Fluence contract it represents as a Swarm address to the WASM file
 * and a requirement of how many nodes will be in the cluster.
 */
export interface App {
    app_address: string,
    storage_receipt: string,
    cluster_size: number,
    developer: string,
    pinToNodes: string[]
}

/**
 * Gets list of enqueued codes from Fluence contract
 */
export async function getEnqueuedApps(contract: Network): Promise<App[]> {

    let unparsedApps = await contract.methods.getEnqueuedApps().call();

    let storageHashes = unparsedApps["0"];
    let storageReceipts = unparsedApps["1"];
    let clusterSizes = unparsedApps["2"];
    let developers = unparsedApps["3"];
    let numberOfPinned: number[] = unparsedApps["4"].map((n) => {return parseInt(n);});
    let allPinned: string[] = unparsedApps["5"];

    return parseCodes(storageHashes, storageReceipts, clusterSizes, developers, numberOfPinned, allPinned);
}

/**
 * Collects codes from response format of Fluence contract.
 */
export function parseCodes(appAddresses: string[],
                    storageReceipts: string[],
                    clusterSizes: string[],
                    developers: string[],
                    numberOfPinned: number[],
                    allPinned: string[]): App[] {
    let count = 0;

    return appAddresses.map((address, index) => {
        let pinned: string[] = [];

        for(var i = 0; i < numberOfPinned[index]; i++){
            pinned.push(allPinned[count]);
            count++;
        }

        return {
            app_address: address,
            storage_receipt: storageReceipts[index],
            cluster_size: parseInt(clusterSizes[index]),
            developer: developers[index],
            pinToNodes: pinned
        };
    });
}
