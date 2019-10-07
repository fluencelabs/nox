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

import cats.data.StateT
import cats.effect.{ConcurrentEffect, Resource, Timer}
import cats.syntax.flatMap._
import cats.syntax.apply._
import fluence.effects.ethclient.EthClient
import fluence.effects.ethclient.data.Block
import fluence.effects.resources.MakeResource
import fluence.log.Log
import fluence.node.config.FluenceContractConfig
import org.web3j.abi.datatypes.generated.Bytes32
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * NodeEth aims to be the sole interaction point with Ethereum for a particular Fluence Node
 *
 * @tparam F Effect
 */
trait NodeEth[F[_]] {

  /**
   * Control events from Ethereum Smart Contract
   */
  def nodeEvents: fs2.Stream[F, NodeEthEvent]

  /**
   * Returns the expected node state, how it's built with received Ethereum data
   */
  def expectedState: F[NodeEthState]

  /**
   * Stream of raw block json, requires ethClient to be configured to keep raw responses!
   */
  def blocksRaw: fs2.Stream[F, String]

  // TODO add a sink for the feedback, so that NodeEth in general could be thought of as a pipe

}

object NodeEth {

  /**
   * Provides the default NodeEth instance
   *
   * @param validatorKey The ValidatorKey of current node (or any node we want to keep an eye on)
   * @param contract FluenceContract
   * @tparam F ConcurrentEffect, used to combine many streams of web3 events
   */
  def apply[F[_]: ConcurrentEffect: Timer](
    validatorKey: ByteVector,
    contract: FluenceContract
  )(implicit log: Log[F]): Resource[F, NodeEth[F]] = {
    val initialState = NodeEthState(validatorKey)

    for {
      stateRef ← MakeResource.refOf[F, NodeEthState](initialState)
      blockQueue ← Resource.liftF(fs2.concurrent.Queue.circularBuffer[F, (Option[String], Block)](8))
      _ ← MakeResource
        .concurrentStream(
          contract.ethClient.blockStream[F]() through blockQueue.enqueue,
          name = "ethClient.blockStream"
        )
    } yield new NodeEth[F] {
      override val nodeEvents: fs2.Stream[F, NodeEthEvent] = fs2.Stream
        .eval(
          contract
            .getAllNodeApps[F](new Bytes32(validatorKey.toArray))
        )
        .flatMap {
          case (allNodeApps, contractAppsLoaded) ⇒
            // TODO: make one filter for all kinds of events, instead of making several separate requests https://github.com/fluencelabs/fluence/issues/463

            // State changes on a new recognized App that should be deployed on this Node
            val onNodeAppS = allNodeApps
              .map(NodeEthState.onNodeApp[F])

            // State changes on App Deleted event
            val onAppDeletedS = contract
              .getAppDeleted[F]
              .map(_.getValue.longValue())
              .map(NodeEthState.onAppDeleted[F])

            // State changes on Node Deleted event
            val onNodeDeletedS = contract.getNodeDeleted
              .map(_.getValue)
              .map(ByteVector(_))
              .map(NodeEthState.onNodeDeleted[F])

            // State changes on New Block
            val onNewBlockS = blockQueue.dequeue.map { case (_, block) => block }
              .map(NodeEthState.onNewBlock[F])

            // State changes on switch from Apps already stored in the contract to App events
            val onContractAppsLoaded =
              fs2.Stream
                .eval(contractAppsLoaded)
                .as(ContractAppsLoaded)
                .map(NodeEthState.onContractAppsLoaded[F])

            // State changes for all kinds of Ethereum events regarding this node
            val stream: fs2.Stream[F, StateT[F, NodeEthState, Seq[NodeEthEvent]]] =
              onNodeAppS merge onAppDeletedS merge onNodeDeletedS merge onNewBlockS merge onContractAppsLoaded

            // Note: state is never being read from the Ref,
            // so no other channels of modifications are allowed
            // TODO handle reorgs
            stream
              .evalMapAccumulate(initialState) {
                case (state, mod) ⇒
                  // Get the new state and a sequence of events, put them to fs2 stream
                  mod.run(state)
              }
              .flatMap {
                case (state, events) ⇒
                  // Save the state to the ref and flatten the events to match the response type
                  fs2.Stream.eval(stateRef.set(state)) *>
                    fs2.Stream.chunk(fs2.Chunk.seq(events))
              }
        }

      /**
       * Returns the expected node state, how it's built with received Ethereum data
       */
      override val expectedState: F[NodeEthState] =
        stateRef.get

      /**
       * Stream of raw block json, requires ethClient to be configured to keep raw responses!
       */
      override val blocksRaw: fs2.Stream[F, String] =
        blockQueue.dequeue.map(_._1).unNone
    }
  }

  /**
   * Loads contract, wraps it with NodeEth
   *
   * @param validatorKey This node's Validator Key
   * @param ethClient To query Ethereum
   * @param config To lookup addresses
   * @return FluenceContract instance with web3j contract inside
   */
  def apply[F[_]: ConcurrentEffect: Timer: Log](
    validatorKey: ByteVector,
    ethClient: EthClient,
    config: FluenceContractConfig
  ): Resource[F, NodeEth[F]] =
    Resource.liftF(FluenceContract(ethClient, config)) >>= (apply[F](validatorKey, _))
}
