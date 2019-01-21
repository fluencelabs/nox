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
import fluence.ethclient.Network.{APPDELETED_EVENT, APPDEPLOYED_EVENT, AppDeployedEventResponse}
import fluence.ethclient.helpers.RemoteCallOps._
import fluence.ethclient.helpers.Web3jConverters.stringToBytes32
import fluence.ethclient.{EthClient, Network}
import fluence.node.config.NodeConfig
import fs2.interop.reactivestreams._
import org.web3j.abi.EventEncoder
import org.web3j.abi.datatypes.generated.{Uint8, _}
import org.web3j.abi.datatypes.{Bool, DynamicArray, Event}
import org.web3j.protocol.core.methods.request.SingleAddressEthFilter
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
   * Builds a filter for specified event. Filter is to be used in eth_newFilter
   *
   * @tparam F Effect, used to query Ethereum for the last block number
   */
  private def eventFilter[F[_]: Async](event: Event) =
    ethClient
      .getBlockNumber[F]
      .map(
        currentBlock ⇒
          new SingleAddressEthFilter(
            DefaultBlockParameter.valueOf(currentBlock.bigInteger),
            DefaultBlockParameterName.LATEST,
            contract.getContractAddress
          ).addSingleTopic(EventEncoder.encode(event))
      )

  /**
   * Returns IDs of the apps hosted by this node's workers
   *
   * @param workerId Tendermint validator key identifying this node
   * @tparam F Effect
   */
  def getNodeAppIds[F[_]](workerId: Bytes32)(implicit F: Async[F]): F[List[Bytes32]] =
    contract
      .getNodeApps(workerId)
      .call[F]
      .flatMap {
        case arr if arr != null && arr.getValue != null => F.point(arr.getValue.asScala.toList)
        case r =>
          F.raiseError[List[Bytes32]](
            new RuntimeException(
              s"Cannot get node apps from the smart contract. Got result '$r'. " +
                s"Are you sure the contract address is correct?"
            )
          )
      }

  /**
   * Returns a finite stream of [[App]] for the current node (specified by `workerId`).
   *
   * @param workerId Tendermint Validator key of current worker, used to filter out apps which aren't related to current node
   * @tparam F Effect
   */
  def getNodeApps[F[_]: Async](workerId: Bytes32): fs2.Stream[F, App] =
    fs2.Stream
      .evalUnChunk(getNodeAppIds[F](workerId).map(cs ⇒ fs2.Chunk(cs: _*)))
      .evalMap(
        appId ⇒
          Apply[F].map2(
            contract
              .getApp(appId)
              .call[F]
              .map(tuple ⇒ (tuple.getValue1, tuple.getValue6, tuple.getValue7)),
            contract
              .getAppWorkers(appId)
              .call[F]
              .map(tuple ⇒ (tuple.getValue1, tuple.getValue2))
          ) {
            case ((storageHash, genesisTime, workerIds), (addrs, ports)) ⇒
              val cluster = Cluster.build(genesisTime, workerIds, addrs, ports, currentWorkerId = workerId)
              cluster.map(App(appId, storageHash, _))
        }
      )
      .unNone

  /**
   * Returns a stream derived from the new AppDeployed events, showing that this node should join new clusters.
   *
   * @param workerId Tendermint Validator key of current worker, used to filter out events which aren't addressed to this node
   * @tparam F ConcurrentEffect to convert Observable into fs2.Stream
   * @return Possibly infinite stream of [[App]]s
   */
  def getNodeAppDeployed[F[_]: ConcurrentEffect](workerId: Bytes32): fs2.Stream[F, App] =
    fs2.Stream
      .eval(eventFilter[F](APPDEPLOYED_EVENT))
      .flatMap(filter ⇒ contract.appDeployedEventFlowable(filter).toStream[F]) // It's checked that current node participates in a cluster there
      .map(FluenceContract.eventToApp(_, workerId))
      .unNone

  /**
   * Returns a stream of [[App]]s already assigned to that node combined with
   * a stream of new [[App]]s coming from AppDeployed events emitted by Fluence Contract
   *
   * @param workerId Tendermint Validator key of current worker, used to filter out events which aren't addressed to this node
   * @tparam F ConcurrentEffect to convert Observable into fs2.Stream
   * @return Possibly infinite stream of [[App]]s
   */
  def getAllNodeApps[F[_]: ConcurrentEffect](workerId: Bytes32): fs2.Stream[F, App] =
    getNodeApps[F](workerId)
      .onFinalize(
        Sync[F].delay(logger.info("Got all the previously prepared clusters. Now switching to the new clusters"))
      ) ++ getNodeAppDeployed(workerId)

  /**
   * Register the node in the contract.
   * TODO check permissions, Ethereum public key should match
   * TODO should not be called anywhere except tests, use CLI to register the node
   *
   * @param nodeConfig Node to add
   * @tparam F Effect
   * @return The block number where transaction has been mined
   */
  def addNode[F[_]: Async](nodeConfig: NodeConfig): F[BigInt] = {
    contract
      .addNode(
        nodeConfig.validatorKey.toBytes32,
        nodeConfig.addressBytes24,
        nodeConfig.startPortUint16,
        nodeConfig.endPortUint16,
        nodeConfig.isPrivateBool
      )
      .call[F]
      .map(_.getBlockNumber)
      .map(BigInt(_))
  }

  /**
   * Publishes a new app to Fluence Network
   *
   * @param storageHash Hash of the code in Swarm
   * @param clusterSize Cluster size required to host this app
   * @tparam F Effect
   * @return The block number where transaction has been mined
   */
  def addApp[F[_]: Async](storageHash: String, clusterSize: Short = 1): F[BigInt] =
    contract
      .addApp(
        stringToBytes32(storageHash),
        stringToBytes32("receipt_stub"),
        new Uint8(clusterSize),
        DynamicArray.empty("bytes32[]").asInstanceOf[DynamicArray[Bytes32]]
      )
      .call[F]
      .map(_.getBlockNumber)
      .map(BigInt(_))

  // TODO: on reconnect, do getApps again and remove all apps that are running on this node but not in getApps list
  // this may happen if we missed some events due to network outage or the like
  /**
   * Returns a stream derived from the new AppDeleted events, showing that an app should be removed.
   *
   * @tparam F ConcurrentEffect to convert Observable into fs2.Stream
   * @return Possibly infinite stream of AppDeleted events
   */
  def getAppDeleted[F[_]: ConcurrentEffect]: fs2.Stream[F, Bytes32] =
    fs2.Stream
      .eval(eventFilter[F](APPDELETED_EVENT))
      .flatMap(filter ⇒ contract.appDeletedEventFlowable(filter).toStream[F])
      .map(_.appID)

  /**
   * Deletes deployed app from contract, triggering AppDeleted event on successful deletion
   * @param appId 32-byte id of the app to be deleted
   * @tparam F Effect
   */
  def deleteApp[F[_]: Async](appId: Bytes32): F[Unit] = contract.deleteApp(appId).call[F].void
}

object FluenceContract {

  /**
   * Tries to convert `AppDeployedEvent` response to [[App]] with all information to launch cluster.
   *
   * @param event event response
   * @param workerId Tendermint Validator key of current worker, used to filter out events which aren't addressed to this node
   * @return Some(App) if current node should host this app, None otherwise
   */
  def eventToApp(
    event: AppDeployedEventResponse,
    workerId: Bytes32
  ): Option[App] = {
    val cluster =
      Cluster.build(event.genesisTime, event.nodeIDs, event.nodeAddresses, event.ports, currentWorkerId = workerId)
    cluster.map(App(event.appID, event.storageHash, _))
  }

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
    import fluence.ethclient.helpers.Web3jConverters.nodeAddressToBytes24
    import nodeConfig._

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
