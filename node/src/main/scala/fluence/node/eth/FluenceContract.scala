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
import fluence.ethclient.Network.{APPDELETED_EVENT, APPDEPLOYED_EVENT, AppDeployedEventResponse, NODEDELETED_EVENT}
import fluence.ethclient.helpers.RemoteCallOps._
import fluence.ethclient.{EthClient, Network}
import fluence.node.eth.conf.FluenceContractConfig
import fluence.node.eth.state.{App, Cluster}
import fs2.interop.reactivestreams._
import org.web3j.abi.EventEncoder
import org.web3j.abi.datatypes.generated._
import org.web3j.abi.datatypes.Event
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
class FluenceContract(private[eth] val ethClient: EthClient, private[eth] val contract: Network)
    extends slogging.LazyLogging {

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
   * @param validatorKey Tendermint validator key identifying this node
   * @tparam F Effect
   */
  private def getNodeAppIds[F[_]](validatorKey: Bytes32)(implicit F: Async[F]): F[List[Bytes32]] =
    contract
      .getNodeApps(validatorKey)
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
   * Returns a finite stream of [[App]] for the current node (determined by `validatorKey`).
   *
   * @param validatorKey Tendermint Validator key of the current node, used to filter out apps which aren't related to current node
   * @tparam F Effect
   */
  private def getNodeApps[F[_]: Async](validatorKey: Bytes32): fs2.Stream[F, state.App] =
    fs2.Stream
      .evalUnChunk(getNodeAppIds[F](validatorKey).map(cs ⇒ fs2.Chunk(cs: _*)))
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
            case ((storageHash, genesisTime, validatorKeys), (addrs, ports)) ⇒
              val cluster = Cluster.build(genesisTime, validatorKeys, addrs, ports, currentValidatorKey = validatorKey)
              cluster.map(App(appId, storageHash, _))
        }
      )
      .unNone

  /**
   * Returns a stream derived from the new AppDeployed events, showing that this node should join new clusters.
   *
   * @param validatorKey Tendermint Validator key of the current node, used to filter out events which aren't addressed to this node
   * @tparam F ConcurrentEffect to convert Observable into fs2.Stream
   * @return Possibly infinite stream of [[App]]s
   */
  private def getNodeAppDeployed[F[_]: ConcurrentEffect](validatorKey: Bytes32): fs2.Stream[F, state.App] =
    fs2.Stream
      .eval(eventFilter[F](APPDEPLOYED_EVENT))
      .flatMap(filter ⇒ contract.appDeployedEventFlowable(filter).toStream[F]) // It's checked that current node participates in a cluster there
      .map(FluenceContract.eventToApp(_, validatorKey))
      .unNone

  /**
   * Returns a stream of [[App]]s already assigned to that node combined with
   * a stream of new [[App]]s coming from AppDeployed events emitted by Fluence Contract
   *
   * @param validatorKey Tendermint Validator key of the current node, used to filter out events which aren't addressed to this node
   * @tparam F ConcurrentEffect to convert Observable into fs2.Stream
   * @return Possibly infinite stream of [[App]]s
   */
  private[eth] def getAllNodeApps[F[_]: ConcurrentEffect](validatorKey: Bytes32): fs2.Stream[F, state.App] =
    getNodeApps[F](validatorKey)
      .onFinalize(
        Sync[F].delay(logger.info("Got all the previously prepared clusters. Now switching to the new clusters"))
      ) ++ getNodeAppDeployed(validatorKey)

  // TODO: on reconnect, do getApps again and remove all apps that are running on this node but not in getApps list
  // this may happen if we missed some events due to network outage or the like
  /**
   * Returns a stream derived from the new AppDeleted events, showing that an app should be removed.
   *
   * @tparam F ConcurrentEffect to convert Observable into fs2.Stream
   * @return Possibly infinite stream of AppDeleted events
   */
  private[eth] def getAppDeleted[F[_]: ConcurrentEffect]: fs2.Stream[F, Bytes32] =
    fs2.Stream
      .eval(eventFilter[F](APPDELETED_EVENT))
      .flatMap(filter ⇒ contract.appDeletedEventFlowable(filter).toStream[F])
      .map(_.appID)

  /**
   * Stream of all the removed node IDs
   *
   * @tparam F ConcurrentEffect to convert Observable into fs2.Stream
   * @return Possibly infinite stream of NodeDeleted events
   */
  private[eth] def getNodeDeleted[F[_]: ConcurrentEffect]: fs2.Stream[F, Bytes32] =
    fs2.Stream
      .eval(eventFilter[F](NODEDELETED_EVENT))
      .flatMap(filter ⇒ contract.nodeDeletedEventFlowable(filter).toStream[F]())
      .map(_.id)

}

object FluenceContract {

  /**
   * Tries to convert `AppDeployedEvent` response to [[App]] with all information to launch cluster.
   *
   * @param event event response
   * @param validatorKey Tendermint Validator key of current node, used to filter out events which aren't addressed to this node
   * @return Some(App) if current node should host this app, None otherwise
   */
  private def eventToApp(
    event: AppDeployedEventResponse,
    validatorKey: Bytes32
  ): Option[state.App] =
    Cluster
      .build(event.genesisTime, event.nodeIDs, event.nodeAddresses, event.ports, currentValidatorKey = validatorKey)
      .map(App(event.appID, event.storageHash, _))

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
}
