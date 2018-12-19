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
 * Status of a Fluence node with all its solvers.
 */
export interface NodeStatus {
    ip: string,
    listOfPorts: string,
    uptime: number,
    numberOfSolvers: number,
    solvers: SolverStatus[]
}

/**
 * If node is not available.
 */
export interface UnavailableNode {
    nodeInfo: Node
}

/**
 * Status of a solver. It can exists but not available or turned off.
 */
export interface SolverStatus {
    SolverRunning?: SolverRunning,
    SolverContainerNotRunning?: SolverContainerNotRunning
    SolverNotYetLaunched?: SolverNotYetLaunched
    SolverHttpCheckFailed?: SolverHttpCheckFailed
}

export interface SolverRunning {
    info: SolverInfo,
    uptime: number
}

export interface SolverContainerNotRunning {
    info: SolverInfo
}

export interface SolverNotYetLaunched {
    info: SolverInfo
}

export interface SolverHttpCheckFailed {
    info: SolverInfo,
    causedBy: string
}

export interface SolverInfo {
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