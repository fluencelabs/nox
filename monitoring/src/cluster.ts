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

export interface ClusterMember {
    id: string,
    port: number
}

export interface Cluster {
    genesis_time: number,
    cluster_members: ClusterMember[]
}

export function parseCluster(genesisTime: number, nodeIds: string[], ports: number[]): Option<Cluster> {
    if (genesisTime !== 0) {
        let clusterMembers: ClusterMember[] = ports.map((port, idx) => {
            return {
                id: nodeIds[idx],
                port: port
            }
        });

        return some({
            genesis_time: genesisTime,
            cluster_members: clusterMembers
        });
    } else { return none }
}