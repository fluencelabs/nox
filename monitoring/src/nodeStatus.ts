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

import {Node} from './node';

/**
 * Status of a Fluence node with all its workers.
 */
export interface NodeStatus {
    ip: string,
    listOfPorts: string,
    uptime: number,
    numberOfWorkers: number,
    workers: WorkerStatus[]
}

/**
 * If node is not available.
 */
export interface UnavailableNode {
    nodeInfo: Node,
    causeBy: string
}

export function isAvailable(status: NodeStatus | UnavailableNode): status is NodeStatus {
    return (<UnavailableNode>status).causeBy === undefined
}

/**
 * Status of a worker. It can exists but not available or turned off.
 */
export interface WorkerStatus {
    WorkerRunning?: WorkerRunning,
    WorkerContainerNotRunning?: WorkerContainerNotRunning
    WorkerNotYetLaunched?: WorkerNotYetLaunched
    WorkerHttpCheckFailed?: WorkerHttpCheckFailed
}

export interface WorkerRunning {
    info: WorkerInfo,
    uptime: number
}

export interface WorkerContainerNotRunning {
    info: WorkerInfo
}

export interface WorkerNotYetLaunched {
    info: WorkerInfo
}

export interface WorkerHttpCheckFailed {
    info: WorkerInfo,
    causedBy: string
}

export interface WorkerInfo {
    clusterId?: string,
    codeId: string,
    lastAppHash?: string,
    lastBlock?: string,
    lastBlockHeight?: number,
    p2pPort: number,
    rpcPort: number,
    stateMachinePrometheusPort?: number,
    tendermintPrometheusPort?: number
}
