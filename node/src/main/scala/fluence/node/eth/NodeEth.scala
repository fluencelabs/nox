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

import cats.effect.ConcurrentEffect
import cats.syntax.functor._
import cats.effect.concurrent.Ref
import fluence.ethclient.EthClient
import fluence.node.eth.conf.FluenceContractConfig
import org.web3j.abi.datatypes.generated.Bytes32
import scodec.bits.ByteVector

import scala.language.higherKinds

trait NodeEth[F[_]] {

  def nodeEvents: fs2.Stream[F, NodeEthEvent]

  def expectedState: F[NodeEthState]

  // Requires raw json to be kept by EthClient
  def blocksRaw: fs2.Stream[F, String]

  // TODO subscribe for events, unchunk them
  def subscribeRaw(): fs2.Stream[F, String] = ???

  // TODO add a sink for the feedback, so that NodeEth in general could be thought of as a pipe

}

object NodeEth {

  def apply[F[_]: ConcurrentEffect](validatorKey: ByteVector, contract: FluenceContract): F[NodeEth[F]] =
    Ref.of(NodeEthState(validatorKey)).map { stateRef ⇒
      // TODO: make singleton streams, do not send the same request to web3j twice
      // TODO ensure resources are cleaned up properly

      new NodeEth[F] {
        override def nodeEvents: fs2.Stream[F, NodeEthEvent] = {
          val runAppWorker = contract
            .getAllNodeApps[F](new Bytes32(validatorKey.toArray))
            .map(RunAppWorker)

          val removeAppWorker = contract
            .getAppDeleted[F]
            .map(_.getValue)
            .map(ByteVector(_))
            .map(RemoveAppWorker)

          val stream = runAppWorker interleave removeAppWorker

          stream.evalTap(ev ⇒ stateRef.update(_.advance(ev)))
        }

        override def expectedState: F[NodeEthState] =
          stateRef.get

        override def blocksRaw: fs2.Stream[F, String] =
          contract.ethClient.blockStream[F].map(_._1).unNone
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
  def apply[F[_]: ConcurrentEffect](
    validatorKey: ByteVector,
    ethClient: EthClient,
    config: FluenceContractConfig
  ): F[NodeEth[F]] =
    apply[F](validatorKey, FluenceContract(ethClient, config))
}
