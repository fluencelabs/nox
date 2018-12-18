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
import {ClustersInfos} from "./cluster";

type UnparsedCodes = { "0": string[]; "1": string[]; "2": string[] };

export interface Code {
    storage_hash: string,
    storage_receipt: string,
    cluster_size: number
}

/**
 * Gets list of enqueued codes from Fluence contract
 */
export async function getEnqueuedCodes(contract: Network): Promise<Code[]> {
    let unparsedCodes = await contract.methods.getEnqueuedCodes().call();
    return parseCodes(unparsedCodes);
}

function parseCodes(unparsed: UnparsedCodes): Code[] {
    let codes: Code[] = [];
    let hashes = unparsed["0"];
    let receipts = unparsed["1"];
    let clusterSizes = unparsed["2"];
    hashes.forEach((hash, index) => {
        let code: Code = {
            storage_hash: hash,
            storage_receipt: receipts[index],
            cluster_size: parseInt(clusterSizes[index])
        };
        codes.push(code);
    });
    return codes;
}

export function parseCodesFromClustersInfos(infos: ClustersInfos): Code[] {
    return parseCodes({"0": infos["2"],
        "1": infos["3"],
        "2": infos["4"]
    });
}
