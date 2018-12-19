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

export interface Code {
    code_address: string,
    storage_receipt: string,
    cluster_size: number,
    developer: string
}

/**
 * Gets list of enqueued codes from Fluence contract
 */
export async function getEnqueuedCodes(contract: Network): Promise<Code[]> {
    let unparsedCodes = await contract.methods.getEnqueuedCodes().call();

    let codeAddresses = unparsedCodes["0"];
    let storageReceipts = unparsedCodes["1"];
    let clusterSizes = unparsedCodes["2"];
    let developers = unparsedCodes["3"];

    return parseCodes(codeAddresses, storageReceipts, clusterSizes, developers);
}

export function parseCodes(codeAddresses: string[],
                    storageReceipts: string[],
                    clusterSizes: string[],
                    developers: string[]): Code[] {
    let codes: Code[] = [];

    codeAddresses.forEach((address, index) => {
        let code: Code = {
            code_address: address,
            storage_receipt: storageReceipts[index],
            cluster_size: parseInt(clusterSizes[index]),
            developer: developers[index]
        };
        codes.push(code);
    });
    return codes;
}
