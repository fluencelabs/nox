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

import {none, Option, some} from "ts-option";

export interface Cluster {
    genesis_time: number,
    node_ids: string[]
}

export function parseCluster(genesisTime: number, nodeIds: string[]): Option<Cluster> {
    if (genesisTime !== 0) {
        return some({
            genesis_time: genesisTime,
            node_ids: nodeIds
        });
    } else { return none }
}
