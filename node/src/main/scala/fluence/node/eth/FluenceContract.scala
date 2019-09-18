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

import cats.{Applicative, Apply, Functor, Monad, Traverse}
import cats.effect._
import cats.effect.concurrent.Deferred
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.apply._
import cats.instances.option._
import fluence.effects.Backoff
import fluence.effects.ethclient.Network.{
  APPDELETED_EVENT,
  APPDEPLOYED_EVENT,
  AppDeployedEventResponse,
  NODEDELETED_EVENT
}
import fluence.effects.ethclient.{EthClient, EthRequestError, Network}
import fluence.effects.ethclient.syntax._
import fluence.log.Log
import fluence.node.config.FluenceContractConfig
import fluence.node.eth.state.{App, Cluster}
import org.web3j.abi.EventEncoder
import org.web3j.abi.datatypes.generated._
import org.web3j.abi.datatypes.{DynamicArray, Event}
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
class FluenceContract(private[eth] val ethClient: EthClient, private[eth] val contract: Network)(
  implicit backoff: Backoff[EthRequestError] = Backoff.default
) {

  /**
   * Builds a filter for specified event. Filter is to be used in eth_newFilter
   *
   * @tparam F Effect, used to query Ethereum for the last block number
   */
  private def eventFilter[F[_]: LiftIO: Monad: Timer](
    event: Event
  ): F[EthFilter] =
    backoff(
      ethClient
        .getBlockNumber[F]
    ).map(
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
  private def getNodeAppIds[F[_]: LiftIO: Timer: Monad: Log](validatorKey: Bytes32): F[List[Uint256]] =
    contract
      .getNodeApps(validatorKey)
      .callUntilSuccess[F]
      .flatMap {
        case arr if arr != null && arr.getValue != null => Applicative[F].point(arr.getValue.asScala.toList)
        case r =>
          Log[F].error(
            s"Cannot get node apps from the smart contract. Got result '$r'. " +
              s"Are you sure the contract address is correct?"
          ) >>
            Timer[F].sleep(Backoff.default.maxDelay) >>
            getNodeAppIds(validatorKey)
      }

  /**
   * Returns a finite stream of [[App]] for the current node (determined by `validatorKey`).
   *
   * @param validatorKey Tendermint Validator key of the current node, used to filter out apps which aren't related to current node
   * @tparam F Effect
   */
  private def getNodeApps[F[_]: LiftIO: Timer: Monad: Log](validatorKey: Bytes32): fs2.Stream[F, state.App] = {
    import org.web3j.tuples.generated.{Tuple2, Tuple8}
    def mapApp(tuple: Tuple8[Bytes32, _, Bytes32, _, _, _, Uint256, DynamicArray[Bytes32]]) = {
      import tuple._
      // storageHash, storageType, genesisTime, validatorKeys
      (component1, component3, component7, component8)
    }
    def mapWorker(tuple: Tuple2[DynamicArray[Bytes24], DynamicArray[Uint16]]) = {
      import tuple._
      // address, ports
      (component1, component2)
    }

    fs2.Stream
      .evalUnChunk(getNodeAppIds[F](validatorKey).map(cs ⇒ fs2.Chunk(cs: _*)))
      .evalMap(
        appId ⇒
          Apply[F]
            .map2(
              contract
                .getApp(appId)
                .callUntilSuccess[F]
                .map(mapApp),
              contract
                .getAppWorkers(appId)
                .callUntilSuccess[F]
                .map(mapWorker)
            ) {
              case ((storageHash, storageType, genesisTime, validatorKeys), (addrs, ports)) ⇒
                val cluster =
                  Cluster.build(genesisTime, validatorKeys, addrs, ports, currentValidatorKey = validatorKey)
                Traverse[Option]
                  .traverse(cluster)(c => App[F](appId, storageHash, storageType, c).value.map(_.toOption))
                  .map(_.flatten)
            }
            .flatten
      )
      .unNone
  }

  /**
   * Returns a stream derived from the new AppDeployed events, showing that this node should join new clusters.
   *
   * @param validatorKey Tendermint Validator key of the current node, used to filter out events which aren't addressed to this node
   * @tparam F ConcurrentEffect to convert Observable into fs2.Stream
   * @return Possibly infinite stream of [[App]]s
   */
  private def getNodeAppDeployed[F[_]: ConcurrentEffect: Timer: Log](validatorKey: Bytes32): fs2.Stream[F, state.App] =
    fs2.Stream
      .eval(eventFilter[F](APPDEPLOYED_EVENT))
      .flatMap(filter ⇒ contract.appDeployedEventFlowable(filter).toStreamRetrying[F]()) // It's checked that current node participates in a cluster there
      .evalMap(FluenceContract.eventToApp[F](_, validatorKey))
      .unNone

  /**
   * Returns a stream of [[App]]s already assigned to that node combined with
   * a stream of new [[App]]s coming from AppDeployed events emitted by Fluence Contract
   *
   * @param validatorKey Tendermint Validator key of the current node, used to filter out events which aren't addressed to this node
   * @tparam F ConcurrentEffect to convert Observable into fs2.Stream
   * @return Possibly infinite stream of [[App]]s
   */
  private[eth] def getAllNodeApps[F[_]: ConcurrentEffect: Timer: Log](
    validatorKey: Bytes32
  ): F[(fs2.Stream[F, state.App], F[Unit])] =
    Deferred[F, Unit].map { switchedToNewApps ⇒
      (getNodeApps[F](validatorKey).onFinalizeCase {
        case ExitCase.Canceled =>
          Log[F].info("Getting all previously prepared clusters canceled.")
        case ExitCase.Completed =>
          switchedToNewApps.complete(()) *>
            Log[F].info("Got all the previously prepared clusters. Now switching to the new clusters.")
        case ExitCase.Error(err) =>
          Log[F].warn(s"Error on getting all previously prepared clusters.", err)
      }.scope ++ getNodeAppDeployed(validatorKey)) -> switchedToNewApps.get
    }

  // TODO: on reconnect, do getApps again and remove all apps that are running on this node but not in getApps list
  // this may happen if we missed some events due to network outage or the like
  /**
   * Returns a stream derived from the new AppDeleted events, showing that an app should be removed.
   *
   * @tparam F ConcurrentEffect to convert Observable into fs2.Stream
   * @return Possibly infinite stream of AppDeleted events
   */
  private[eth] def getAppDeleted[F[_]: ConcurrentEffect: Timer: Log]: fs2.Stream[F, Uint256] =
    fs2.Stream
      .eval(eventFilter[F](APPDELETED_EVENT))
      .flatMap(filter ⇒ contract.appDeletedEventFlowable(filter).toStreamRetrying[F]())
      .map(_.appID)

  /**
   * Stream of all the removed node IDs
   *
   * @tparam F ConcurrentEffect to convert Observable into fs2.Stream
   * @return Possibly infinite stream of NodeDeleted events
   */
  private[eth] def getNodeDeleted[F[_]: ConcurrentEffect: Timer: Log]: fs2.Stream[F, Bytes32] =
    fs2.Stream
      .eval(eventFilter[F](NODEDELETED_EVENT))
      .flatMap(filter ⇒ contract.nodeDeletedEventFlowable(filter).toStreamRetrying[F]())
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
  private def eventToApp[F[_]: Functor: Applicative](
    event: AppDeployedEventResponse,
    validatorKey: Bytes32
  ): F[Option[state.App]] =
    Traverse[Option]
      .traverse(
        Cluster
          .build(event.genesisTime, event.nodeIDs, event.nodeAddresses, event.ports, currentValidatorKey = validatorKey)
      )(c => App[F](event.appID, event.storageHash, event.storageType, c).value.map(_.toOption))
      .map(_.flatten)

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
