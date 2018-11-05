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

pragma solidity ^0.4.24;

// TODO: comply with security suggestions from: https://github.com/OpenZeppelin/openzeppelin-solidity

// TODO: add pausing, circuit-breaking logic 

// TODO: should this contract accept money? 
// if no, reject payments. 
// if yes, is it possible to introduce balance limit to avoid becoming high-profile contract? and thus target for attacks

// TODO: what are most critical invariants here?
// should we put a bug-bounty on them?

// TODO: what are gas usage goals/targets? is there any limit?
// TODO: calculate current gas usage
// TODO: better control codes.length so we don't exceed gasLimit
// TODO: should code hash be hash of the `storageHash`? so no one could download it. in other words, is code private?


// implementation is at https://github.com/OpenZeppelin/openzeppelin-solidity/blob/master/contracts/access/Whitelist.sol
// example tests are at https://github.com/OpenZeppelin/openzeppelin-solidity/blob/master/test/ownership/Whitelist.test.js
//import "openzeppelin-solidity/contracts/access/Whitelist.sol";

contract Deployer {
    struct Solver {
        bytes32 id;
        bytes32 nodeAddress;
    }

    struct Code {
        bytes32 storageHash;
        bytes32 storageReceipt;
        uint8 clusterSize;
    }

    struct BusyCluster {
        bytes32 clusterID;
        Code code;
        bytes32[] solvers;
    }

    // Emitted when there is enough free Solvers for some Code
    // Solvers should form a cluster in reaction to this event
    event ClusterFormed(bytes32 clusterID, bytes32[] solverIDs, bytes32[] solverAddrs);

    // Emitted when Code is enqueued, telling that there is not enough Solvers yet
    event CodeEnqueued(bytes32 storageHash);

    // Emitted on every new Solver
    event NewSolver(bytes32 id);

    // Solvers ready to work
    Solver[] private freeSolvers;

    // Last cluster used this solver. Used to deduplicate cluster participants
    mapping(bytes32 => bytes32) private lastClusterForSolverID;

    // Number of added freeSolvers for each solver ID
    mapping(bytes32 => uint) private solverCountsByID;

    // Number of free solvers with pairwise distinct IDs (number of non-zero values in solverCountsById)
    uint distinctSolverCount = 0;

    // Cluster with assigned Code
    mapping(bytes32 => BusyCluster) private busyClusters;

    // Number of existing clusters, used for clusterID generation
    // starting with 1, so we could check existince of cluster in the mapping, e.g:
    // if (busyCluster[someId].clusterID > 0)
    uint256 clusterCount = 1;

    // Codes waiting for solvers
    Code[] private enqueuedCodes;

    /** @dev Adds solver to the work-waiting queue
      * @param solverID some kind of unique ID
      * @param solverAddress currently packed (TendermintP2PKey, IP, port) tuple, subject to change
      * emits NewSolver event containing number of free solvers, subject to change
      * emits ClusterFormed event when there is enough solvers for some Code
      */
    function addSolver(bytes32 solverID, bytes32 solverAddress) external {
        freeSolvers.push(Solver(solverID, solverAddress));

        if (solverCountsByID[solverID] == 0) {
            distinctSolverCount++;
        }
        solverCountsByID[solverID]++;

        emit NewSolver(solverID);
        matchWork();
    }

    /** @dev Adds new Code to be deployed on Solvers when there are enough of them
      * @param storageHash Swarm storage hash; allows code distributed and downloaded through it
      * @param storageReceipt Swarm receipt, serves as a proof that code is stored
      * @param clusterSize specifies number of Solvers that must serve Code
      * emits ClusterFormed event when there is enough solvers for the Code and emits CodeEnqueued otherwise, subject to change
      */
    function addCode(bytes32 storageHash, bytes32 storageReceipt, uint8 clusterSize) external {
        enqueuedCodes.push(Code(storageHash, storageReceipt, clusterSize));
        if (!matchWork()) {
            emit CodeEnqueued(storageHash);
        }
    }

    /** @dev Allows anyone with clusterID to retrieve assigned Code
     * @param clusterID unique id of cluster (keccak256 of abi.encodePacked(clusterIDs))
     */
    function getCluster(bytes32 clusterID) external view returns (bytes32, bytes32, bytes32[]) {
        BusyCluster memory cluster = busyClusters[clusterID];
        require(cluster.clusterID > 0, "there is no such cluster");
        return (cluster.code.storageHash, cluster.code.storageReceipt, cluster.solvers);
    }

    /** @dev Allows to track contract status
     */
    function getStatus() external view returns (uint8, uint256, uint256[]) {
        uint256[] memory cs = new uint256[](enqueuedCodes.length);
        for (uint j = 0; j < enqueuedCodes.length; ++j) {
            cs[j] = enqueuedCodes[j].clusterSize;
        }
        // fast way to check that contract deployed incorrectly: in this case getStatus() returns (0, 0, [])
        uint8 version = 101;
        return (version, freeSolvers.length, cs);
    }

    /** @dev Checks if there is enough free Solvers for undeployed Code
     * emits ClusterFormed event if so
     */
    function matchWork() internal returns (bool) {
        // TODO: To avoid initially wrong p2p configuration we also need to ensure that the information packed in solver
        // address (Tendermint p2p keys addresses and host/port pairs) is also pairwise distinct in a formed cluster.

        // looking for a enqueued code for which we have enough solvers
        for (uint idx = 0; idx < enqueuedCodes.length; ++idx) {
            if (distinctSolverCount >= enqueuedCodes[idx].clusterSize) {
                break;
            }
        }

        if (idx >= enqueuedCodes.length) {
            // means that we passed through codes array without hitting condition
            return false;
        }

        // match found
        Code memory code = enqueuedCodes[idx];

        // replace this code in enqueuedCodes with the last one
        if (idx + 1 != enqueuedCodes.length) {
            enqueuedCodes[idx] = enqueuedCodes[enqueuedCodes.length - 1];
        }
        --enqueuedCodes.length;

        // preparing response data
        bytes32[] memory clusterSolverIDs = new bytes32[](code.clusterSize);
        bytes32[] memory clusterSolverAddrs = new bytes32[](code.clusterSize);
        bytes32 clusterID = bytes32(clusterCount++);

        uint solversToCollect = code.clusterSize;
        // traversing freeSolvers and collecting cluster participants from them

        for (uint j = 0; j < freeSolvers.length; ) {
            if (solversToCollect == 0) {
                break;
            }
            // lastClusterForSolverID keeps the last clusterID for solver thus avoiding duplicate solver IDs in cluster
            if (lastClusterForSolverID[freeSolvers[j].id] != clusterID) {
                --solversToCollect;
                clusterSolverIDs[solversToCollect] = freeSolvers[j].id;
                clusterSolverAddrs[solversToCollect] = freeSolvers[j].nodeAddress;

                // update lastClusterForSolverID, solverCountsByID and distinctSolverCount
                lastClusterForSolverID[freeSolvers[j].id] = clusterID;
                assert(solverCountsByID[freeSolvers[j].id] > 0);
                if (--solverCountsByID[freeSolvers[j].id] == 0) {
                    --distinctSolverCount;
                }

                // replace this solver with the last one
                if (j + 1 != freeSolvers.length) {
                    freeSolvers[j] = freeSolvers[freeSolvers.length - 1];
                }
                --freeSolvers.length;
            }
            else {
                ++j;
            }
        }
        assert(solversToCollect == 0);

        busyClusters[clusterID] = BusyCluster(clusterID, code, clusterSolverIDs);

        emit ClusterFormed(clusterID, clusterSolverIDs, clusterSolverAddrs);
        return true;
    }
}
