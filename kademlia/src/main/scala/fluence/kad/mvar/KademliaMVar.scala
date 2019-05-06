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
import cats.data.StateT
import cats.effect._
import cats.kernel.Monoid
import cats.syntax.functor._
import fluence.kad._
import fluence.kad.core.{Siblings, SiblingsState}
import fluence.kad.protocol.{KademliaRpc, Key, Node}

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
          P.parallel(KademliaMVar.siblingsOps[F, C](nodeId, conf.maxSiblingsSize)),
          P.parallel(MVarBucketOps[F, C](conf.maxBucketSize))
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

  /**
   * Builds asynchronous sibling ops with $maxSiblings nodes max.
   *
   * @param nodeId      Siblings are sorted by distance to this nodeId
   * @param maxSiblings Max number of closest siblings to store
   * @tparam C Node contacts type
   */
  private def siblingsOps[F[_]: Concurrent, C](nodeId: Key, maxSiblings: Int): F[SiblingsState[F, C]] =
    ReadableMVar.of(Siblings[C](nodeId, maxSiblings)).map { state ⇒
      new SiblingsState[F, C] {

        override protected def run[T](mod: StateT[F, Siblings[C], T]): F[T] =
          state.apply(mod)

        override def read: F[Siblings[C]] =
          state.read
      }
    }
}
