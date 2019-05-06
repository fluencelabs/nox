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

package fluence.kad.mvar

import cats.Parallel
import cats.effect._
import cats.kernel.Monoid
import cats.syntax.functor._
import fluence.kad._
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import fluence.kad.state.{BucketsState, ReadableMVar, Siblings, SiblingsState}

import scala.language.{higherKinds, implicitConversions}

// TODO: write unit tests
object KademliaMVar {

  /**
   * Kademlia service to be launched as a singleton on local node.
   *
   * @param nodeId        Current node ID
   * @param contact       Node's contact to advertise
   * @param rpcForContact Getter for RPC calling of another nodes
   * @param conf          Kademlia conf
   * @param checkNode     Node could be saved to RoutingTable only if checker returns F[ true ]
   * @tparam C Contact info
   */
  def apply[F[_]: ConcurrentEffect: Timer, P[_], C](
    nodeId: Key,
    contact: IO[C],
    rpcForContact: C ⇒ KademliaRpc[C],
    conf: KademliaConf,
    checkNode: Node[C] ⇒ IO[Boolean]
  )(implicit P: Parallel[F, P]): F[Kademlia[F, C]] =
    P.sequential(
        P.apply.product(
          P.parallel(SiblingsState.withMVar[F, C](nodeId, conf.maxSiblingsSize)),
          P.parallel(BucketsState.withMVar[F, C](conf.maxBucketSize))
        )
      )
      .map {
        case (siblings, buckets) ⇒
          // TODO run kad maintenance using timer

          Kademlia[F, P, C](
            nodeId,
            conf.parallelism,
            conf.pingExpiresIn,
            checkNode,
            contact.map(c ⇒ Node(nodeId, c)),
            rpcForContact,
            buckets,
            siblings
          )
      }

  /**
   * Builder for client-side implementation of KademliaMVar
   *
   * @param rpc       Getter for RPC calling of another nodes
   * @param conf      Kademlia conf
   * @param checkNode Node could be saved to RoutingTable only if checker returns F[ true ]
   * @tparam C Contact info
   */
  def client[F[_]: ConcurrentEffect: Timer, P[_], C](
    rpc: C ⇒ KademliaRpc[C],
    conf: KademliaConf,
    checkNode: Node[C] ⇒ IO[Boolean]
  )(implicit P: Parallel[F, P]): F[Kademlia[F, C]] =
    apply[F, P, C](
      Monoid.empty[Key],
      IO.raiseError(new IllegalStateException("Client may not have a Contact")),
      rpc,
      conf,
      checkNode
    )
}
