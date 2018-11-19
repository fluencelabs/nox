/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.node.eth

import cats.effect.{Async, ConcurrentEffect}
import cats.syntax.functor._
import fluence.ethclient.Deployer.{CLUSTERFORMED_EVENT, ClusterFormedEventResponse}
import fluence.ethclient.{Deployer, EthClient}
import fluence.node.NodeConfig
import fluence.node.tendermint.ClusterData
import org.web3j.abi.EventEncoder
import org.web3j.protocol.core.{DefaultBlockParameter, DefaultBlockParameterName}
import org.web3j.protocol.core.methods.request.EthFilter
import fluence.ethclient.helpers.RemoteCallOps._
import fluence.ethclient.helpers.JavaRxToFs2._
import org.web3j.abi.datatypes.DynamicArray
import org.web3j.abi.datatypes.generated.{Bytes24, Bytes32, Uint16, Uint256}
import org.web3j.tuples.generated

import scala.collection.JavaConverters._
import scala.language.higherKinds

/**
 * DeployerContract wraps all the functionality necessary for working with Deployer contract over Ethereum.
 *
 * @param ethClient Ethereum client
 * @param deployer Deployer contract ABI, received from Ethereum
 */
class DeployerContract(private val ethClient: EthClient, private val deployer: Deployer) {
  import DeployerContract.NodeConfigEthOps

  /**
   * Builds an actual filter for CLUSTERFORMED event.
   *
   * @tparam F Effect, used to query Ethereum for the last block number
   */
  private def clusterFormedFilter[F[_]: Async]: F[EthFilter] =
    ethClient
      .getBlockNumber[F]
      .map(
        currentBlock ⇒
          new EthFilter(
            DefaultBlockParameter.valueOf(currentBlock.bigInteger),
            DefaultBlockParameterName.LATEST,
            deployer.getContractAddress
          ).addSingleTopic(EventEncoder.encode(CLUSTERFORMED_EVENT))
      )

  /**
   * Returns IDs of the clusters the given node participates in.
   *
   * @param nodeConfig Node to pick validatorKey from to lookup the clusters for
   * @tparam F Effect
   */
  def getNodeClusterIds[F[_]: Async](nodeConfig: NodeConfig): F[List[Bytes32]] =
    deployer
      .getNodeClusters(nodeConfig.validatorKeyBytes32)
      .call[F]
      .map(_.getValue.asScala.toList)

  /**
   * Returns a finite stream of ClusterData for the given node.
   *
   * @param nodeConfig Node to pick validatorKey from to lookup the clusters for
   * @tparam F Effect
   */
  def getNodeClusters[F[_]: Async](nodeConfig: NodeConfig): fs2.Stream[F, ClusterData] =
    fs2.Stream
      .evalUnChunk(getNodeClusterIds[F](nodeConfig).map(cs ⇒ fs2.Chunk(cs: _*)))
      .evalMap(
        clusterId ⇒
          deployer
            .getCluster(clusterId)
            .call[F]
            .map(DeployerContract.clusterTupleToClusterFormed(clusterId, _))
      )
      .map(ClusterData.fromClusterFormedEvent(_, nodeConfig))
      .unNone

  /**
   * Returns a stream derived from the new ClusterFormed events, showing that this node should join new clusters.
   *
   * @param nodeConfig Node to pick validatorKey from to lookup the clusters for
   * @tparam F ConcurrentEffect to convert Observable into fs2.Stream
   * @return Possibly infinite stream of ClusterData
   */
  def getNodeClustersFormed[F[_]: ConcurrentEffect](nodeConfig: NodeConfig): fs2.Stream[F, ClusterData] =
    fs2.Stream
      .eval(clusterFormedFilter[F])
      .flatMap(filter ⇒ deployer.clusterFormedEventObservable(filter).toFS2[F]) // TODO: we should filter by verifier id
      .map(ClusterData.fromClusterFormedEvent(_, nodeConfig))
      .unNone

  /**
   * Returns a combined stream of clusters where this node should already participate and the new ones coming from
   * ClusterFormed events.
   *
   * @param nodeConfig Node to pick validatorKey from to lookup the clusters for
   * @tparam F ConcurrentEffect to convert Observable into fs2.Stream
   * @return Possibly infinite stream of ClusterData
   */
  def getAllNodeClusters[F[_]: ConcurrentEffect](nodeConfig: NodeConfig): fs2.Stream[F, ClusterData] =
    getNodeClusters[F](nodeConfig) ++ getNodeClustersFormed(nodeConfig)

  /**
   * Register the node with the Deployer contract.
   * TODO check permissions, Ethereum public key should match
   *
   * @param nodeConfig Node to add
   * @tparam F Effect
   * @return The block number where transaction has been mined
   */
  def addNode[F[_]: Async](nodeConfig: NodeConfig): F[BigInt] =
    deployer
      .addNode(
        nodeConfig.validatorKeyBytes32,
        nodeConfig.addressBytes24,
        nodeConfig.startPortUint16,
        nodeConfig.endPortUint16
      )
      .call[F]
      .map(_.getBlockNumber)
      .map(BigInt(_))
}

object DeployerContract {

  /**
   * Corresponds to return type for Deployer.getCluster method.
   */
  type ContractClusterTuple = generated.Tuple6[
    Bytes32,
    Bytes32,
    Uint256,
    DynamicArray[Bytes32],
    DynamicArray[Bytes24],
    DynamicArray[Uint16]
  ]

  /**
   * Provides DeployerContract
   * @param ethClient To query Ethereum
   * @param config To lookup addresses
   * @return DeployerContract
   */
  def apply(ethClient: EthClient, config: DeployerContractConfig): DeployerContract =
    new DeployerContract(
      ethClient,
      ethClient.getContract(
        config.deployerContractAddress,
        config.deployerContractOwnerAccount,
        Deployer.load
      )
    )

  // Tuple corresponds to the data structure of Deployer contract array
  private def clusterTupleToClusterFormed(
    clusterId: Bytes32,
    clusterTuple: ContractClusterTuple
  ): ClusterFormedEventResponse = {
    val artificialEvent = new ClusterFormedEventResponse()
    artificialEvent.clusterID = clusterId
    artificialEvent.storageHash = clusterTuple.getValue1
    artificialEvent.genesisTime = clusterTuple.getValue3
    artificialEvent.solverIDs = clusterTuple.getValue4
    artificialEvent.solverAddrs = clusterTuple.getValue5
    artificialEvent.solverPorts = clusterTuple.getValue6
    artificialEvent
  }

  implicit class NodeConfigEthOps(nodeConfig: NodeConfig) {
    import fluence.ethclient.helpers.Web3jConverters.{base64ToBytes32, solverAddressToBytes24}
    import nodeConfig._

    /**
     * Returns node's public key in format ready to pass to the contract.
     */
    def validatorKeyBytes32: Bytes32 = base64ToBytes32(validatorKey.value)

    /**
     * Returns node's address information (host, Tendermint p2p key) in format ready to pass to the contract.
     */
    def addressBytes24: Bytes24 = solverAddressToBytes24(ip, nodeAddress)

    /**
     * Returns starting port as uint16.
     */
    def startPortUint16: Uint16 = new Uint16(startPort)

    /**
     * Returns ending port as uint16.
     */
    def endPortUint16: Uint16 = new Uint16(endPort)
  }
}
