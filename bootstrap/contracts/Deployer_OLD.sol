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

// implementation is at https://github.com/OpenZeppelin/openzeppelin-solidity/blob/master/contracts/access/Whitelist.sol
// example tests are at https://github.com/OpenZeppelin/openzeppelin-solidity/blob/master/test/ownership/Whitelist.test.js
//import "openzeppelin-solidity/contracts/access/Whitelist.sol";

contract Deployer_OLD {
    struct Solver {
        bytes32 id;
        bytes32 nodeAddress;
    }

    struct Code {
        // TODO: should storageHash be of type hash?
        bytes32 storageHash;
        bytes32 storageReceipt;
        uint8 clusterSize;

        bool deployed;

        // TODO: should there be more statuses to just "deployed or not"?
        // e.g 'deploying', 'deployed'
        // maybe how many times it gets deployed, if that's the case
        
        // TODO: there should be timeout on deployment status, and it should be confirmed periodically
        // cuz it is possible for Solvers to ignore `CodeDeploying` while code is marked as deployed=true
    }

    struct BusyCluster {
        bytes32 clusterID;
        Code code;
    }

    // Emitted when there is enough free Solvers for some Code
    // Solvers should form a cluster in reaction to this event
    event ClusterFormed(bytes32 clusterID, bytes32[] solverIDs, bytes32[] solverAddrs);

    // Emitted when Code is enqueued, telling that there is not enough Solvers yet
    event CodeEnqueued(bytes32 storageHash);

    // Emitted on every new Solver
    event NewSolver(bytes32 id);

    // Solvers enqueued for work
    bytes32[] private freeSolvers;

    // All solvers
    mapping(bytes32 => Solver) private solvers;

    // Cluster with assigned Code
    mapping(bytes32 => BusyCluster) private busyClusters;
    
    // Number of existing clusters, used for clusterID generation
    // starting with 1, so we could check existince of cluster in the mapping, e.g:
    // if (busyCluster[someId].clusterID > 0)
    uint256 clustersCount = 1;

    // Deployed and undeployed Codes
    Code[] private codes;

    /** @dev Adds solver to the work-waiting queue
      * @param solverID some kind of unique ID
      * @param solverAddress currently IP address, subject to change
      * emits NewSolver event containing number of free solvers, subject to change
      * emits ClusterFormed event when there is enough solvers for some Code
      */
    function addSolver(bytes32 solverID, bytes32 solverAddress) external {
        require(solvers[solverID].id == 0, "This solver is already registered");
        solvers[solverID] = Solver(solverID, solverAddress);
        freeSolvers.push(solverID);
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
        codes.push(Code(storageHash, storageReceipt, clusterSize, false));
        if (!matchWork()) {
            // TODO: should it be hash of the `storageHash`? so no one could download it
            // in other words, is code private?
            emit CodeEnqueued(storageHash);
        }
    }

    /** @dev Allows anyone with clusterID to retrieve assigned Code
     * @param clusterID unique id of cluster (keccak256 of abi.encodePacked(clusterIDs))
     */
    function getCode(bytes32 clusterID) external view returns (bytes32, bytes32) {
        BusyCluster memory cluster = busyClusters[clusterID];
        require(cluster.clusterID > 0, "there is no such cluster");
        return (cluster.code.storageHash, cluster.code.storageReceipt);
    }

    function getStatus() external view returns (uint256, uint256, uint256[]) {
        uint256[] memory cs = new uint256[](codes.length);
        for (uint j = 0; j < codes.length; j++)
            cs[j] = codes[j].deployed ? codes[j].clusterSize : 0;
        return (freeSolvers.length, codes.length, cs);
    }

    /** @dev Checks if there is enough free Solvers for undeployed Code
     * emits ClusterFormed event if so
     */
    function matchWork() internal returns (bool) {
        uint idx = 0;
        // TODO: better control codes.length so we don't exceed gasLimit
        // maybe separate deployed and undeployed code in two arrays
        for (; idx < codes.length; ++idx) {
            if (freeSolvers.length >= codes[idx].clusterSize && !codes[idx].deployed) {
                break;
            }
        }

        // check if we hit the condition `freeSolvers.length >= codes[idx].clusterSize` above
        // idx >= codes.length means that we skipped through codes array without hitting condition
        if (idx >= codes.length) {
            return false;
        }
        
        Code storage code = codes[idx];
        bytes32[] memory cluster = new bytes32[](code.clusterSize);
        bytes32[] memory clusterAddrs = new bytes32[](code.clusterSize);
        for (uint j = 0; j < code.clusterSize; j++) {
            // moving & deleting from the end, so we don't have gaps
            // yep, that makes the solvers[] a LIFO, but is it a problem really?
            bytes32 solverID = freeSolvers[freeSolvers.length - j - 1];
            cluster[j] = solverID;
            clusterAddrs[j] = solvers[solverID].nodeAddress;
        }
        freeSolvers.length -= code.clusterSize; // TODO: that's awful, but Solidity doesn't change array length on delete
        bytes32 clusterID = bytes32(clustersCount++);
        busyClusters[clusterID] = BusyCluster(clusterID, code);

        code.deployed = true;
        emit ClusterFormed(clusterID, cluster, clusterAddrs);
        return true;
    }
}
