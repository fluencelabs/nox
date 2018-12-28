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

import cats.Apply
import cats.effect.{Async, ConcurrentEffect, Sync}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.ethclient.Network.{CLUSTERFORMED_EVENT, ClusterFormedEventResponse}
import fluence.ethclient.helpers.JavaRxToFs2._
import fluence.ethclient.helpers.RemoteCallOps._
import fluence.ethclient.helpers.Web3jConverters.stringToBytes32
import fluence.ethclient.{EthClient, Network}
import fluence.node.config.NodeConfig
import fluence.node.tendermint.ClusterData
import org.web3j.abi.EventEncoder
import org.web3j.abi.datatypes.generated.{Uint8, _}
import org.web3j.abi.datatypes.{Bool, DynamicArray}
import org.web3j.protocol.core.methods.request.{EthFilter, SingleAddressEthFilter}
import org.web3j.protocol.core.{DefaultBlockParameter, DefaultBlockParameterName}

import scala.collection.JavaConverters._
import scala.language.higherKinds

/**
 * FluenceContract wraps all the functionality necessary for working with Fluence contract over Ethereum.
 *
 * @param ethClient Ethereum client
 * @param contract Contract ABI, received from Ethereum
 */
class FluenceContract(private val ethClient: EthClient, private val contract: Network) extends slogging.LazyLogging {
  import FluenceContract.NodeConfigEthOps

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
          new SingleAddressEthFilter(
            DefaultBlockParameter.valueOf(currentBlock.bigInteger),
            DefaultBlockParameterName.LATEST,
            contract.getContractAddress
          ).addSingleTopic(EventEncoder.encode(CLUSTERFORMED_EVENT))
      )

  /**
   * Returns IDs of the clusters the given node participates in.
   *
   * @param nodeConfig Node to pick validatorKey from to lookup the clusters for
   * @tparam F Effect
   */
  def getNodeClusterIds[F[_]](nodeConfig: NodeConfig)(implicit F: Async[F]): F[List[Bytes32]] =
    contract
      .getNodeClusters(nodeConfig.validatorKeyBytes32)
      .call[F]
      .flatMap {
        case arr if arr != null && arr.getValue != null => F.point(arr.getValue.asScala.toList)
        case r =>
          F.raiseError[List[Bytes32]](
            new RuntimeException(
              s"Cannot get node clusters from the smart contract. Got result '$r'. " +
                s"Are you sure the contract address is correct?"
            )
          )
      }

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
          Apply[F].map2(
            contract
              .getCluster(clusterId)
              .call[F]
              .map(tuple ⇒ (tuple.getValue1, tuple.getValue6, tuple.getValue7)),
            contract
              .getClusterWorkers(clusterId)
              .call[F]
              .map(tuple ⇒ (tuple.getValue1, tuple.getValue2))
          ) {
            case ((storageHash, genesisTime, nodeIds), (addrs, ports)) ⇒
              ClusterData.build(clusterId, nodeIds, genesisTime, storageHash, addrs, ports, nodeConfig)
        }
      )
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
      .flatMap(filter ⇒ contract.clusterFormedEventFlowable(filter).toFS2[F]) // TODO: we should filter by verifier id! Now node will join all the clusters
      .map(FluenceContract.eventToClusterData(_, nodeConfig))
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
    getNodeClusters[F](nodeConfig)
      .onFinalize(
        Sync[F].delay(logger.info("Got all the previously prepared clusters. Now switching to the new clusters"))
      ) ++ getNodeClustersFormed(nodeConfig)

  /**
   * Register the node in the contract.
   * TODO check permissions, Ethereum public key should match
   * TODO should not be called from scala, use CLI to register the node
   *
   * @param nodeConfig Node to add
   * @tparam F Effect
   * @return The block number where transaction has been mined
   */
  def addNode[F[_]: Async](nodeConfig: NodeConfig): F[BigInt] =
    contract
      .addNode(
        nodeConfig.validatorKeyBytes32,
        nodeConfig.addressBytes24,
        nodeConfig.startPortUint16,
        nodeConfig.endPortUint16,
        nodeConfig.isPrivateBool
      )
      .call[F]
      .map(_.getBlockNumber)
      .map(BigInt(_))

  /**
   * Adds a new code to be launched with a new cluster
   *
   * TODO should not be called from scala
   * @param code Code app name
   * @param clusterSize Cluster size
   * @tparam F Effect
   * @return The block number where transaction has been mined
   */
  def addApp[F[_]: Async](code: String, clusterSize: Short = 1): F[BigInt] =
    contract
      .addApp(
        stringToBytes32(code),
        stringToBytes32("receipt_stub"),
        new Uint8(clusterSize),
        DynamicArray.empty("bytes32[]").asInstanceOf[DynamicArray[Bytes32]]
      )
      .call[F]
      .map(_.getBlockNumber)
      .map(BigInt(_))
}

object FluenceContract {

  /**
   * Tries to convert `ClusterFormedEvent` response to [[ClusterData]] with all information to launch cluster.
   *
   * @param event event response
   * @param nodeConfig information about current node
   * @return true if provided node key belongs to the cluster from the event
   */
  def eventToClusterData(
    event: ClusterFormedEventResponse,
    nodeConfig: NodeConfig
  ): Option[ClusterData] =
    ClusterData.build(
      event.clusterID,
      event.nodeIDs,
      event.genesisTime,
      event.storageHash,
      event.nodeAddresses,
      event.ports,
      nodeConfig
    )

  /**
   * Loads contract
   *
   * @param ethClient To query Ethereum
   * @param config To lookup addresses
   * @return FluenceContract instance with web3j contract inside
   */
  def apply(ethClient: EthClient, config: FluenceContractConfig): FluenceContract =
    new FluenceContract(
      ethClient,
      ethClient.getContract[Network](
        config.address,
        config.ownerAccount,
        Network.load
      )
    )

  implicit class NodeConfigEthOps(nodeConfig: NodeConfig) {
    import fluence.ethclient.helpers.Web3jConverters.{base64ToBytes32, nodeAddressToBytes24}
    import nodeConfig._

    /**
     * Returns node's public key in format ready to pass to the contract.
     */
    def validatorKeyBytes32: Bytes32 = base64ToBytes32(validatorKey.value)

    /**
     * Returns node's address information (host, Tendermint p2p key) in format ready to pass to the contract.
     */
    def addressBytes24: Bytes24 = nodeAddressToBytes24(endpoints.ip.getHostAddress, nodeAddress)

    /**
     * Returns starting port as uint16.
     */
    def startPortUint16: Uint16 = new Uint16(endpoints.minPort)

    /**
     * Returns ending port as uint16.
     */
    def endPortUint16: Uint16 = new Uint16(endpoints.maxPort)

    def isPrivateBool: Bool = new Bool(isPrivate)
  }
}
