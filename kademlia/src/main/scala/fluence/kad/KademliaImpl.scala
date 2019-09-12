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

package fluence.kad

import cats.{Monad, Parallel}
import cats.data.EitherT
import cats.syntax.applicative._
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.eq._
import cats.effect.Clock
import fluence.kad.contact.ContactAccess
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import fluence.kad.routing.RoutingTable
import fluence.log.Log

import scala.language.higherKinds

/**
 * Kademlia implementation for current node and all Kademlia-related RPC calls, both incoming and outgoing
 *
 * @param nodeKey        Current node's Kademlia key
 * @param parallelism   Parallelism factor (named Alpha in paper)
 * @tparam F Effect
 * @tparam C Contact info
 */
class KademliaImpl[F[_]: Monad: Clock: Parallel, C](
  override val nodeKey: Key,
  parallelism: Int,
  ownContactGetter: F[Node[C]],
  routing: RoutingTable[F, C]
)(implicit ca: ContactAccess[F, C])
    extends Kademlia[F, C] {
  self ⇒

  override protected def rpc(contact: C): KademliaRpc[F, C] =
    ContactAccess[F, C].rpc(contact)

  override val ownContact: F[Node[C]] = ownContactGetter

  override def update(node: Node[C])(implicit log: Log[F]): F[Boolean] =
    routing.state.update(node).map(_.updated.contains(node.key))

  override def handleRPC: KademliaRpc[F, C] =
    new LocalRpc(ownContactGetter, routing.local)

  override def findNode(key: Key, maxRequests: Int)(implicit log: Log[F]): F[Option[Node[C]]] =
    routing.local.find(key).flatMap {
      case found @ Some(_) ⇒ (found: Option[Node[C]]).pure[F]

      case None ⇒
        callIterative[String, Unit](
          key,
          n ⇒ EitherT.cond[F](n.key === key, (), "Mismatching node"),
          numToCollect = 1,
          maxCalls = maxRequests
        ).map(_.headOption.map(_._1))
    }

  override def lookupIterative(key: Key, numberOfNodes: Int)(implicit log: Log[F]): F[Seq[Node[C]]] =
    routing.iterative.lookupIterative(key, numberOfNodes, parallelism)

  override def callIterative[E, A](
    key: Key,
    fn: Node[C] ⇒ EitherT[F, E, A],
    numToCollect: Int,
    maxCalls: Int,
    isIdempotentFn: Boolean = true
  )(implicit log: Log[F]): F[Seq[(Node[C], A)]] =
    routing.iterative
      .callIterative(key, fn, numToCollect, parallelism, maxCalls, isIdempotentFn)
      .map(_.toSeq)

  override def join(peers: Seq[C], neighbors: Int)(implicit log: Log[F]): F[Boolean] =
    routing.iterative.join(peers, neighbors, parallelism).value.map(_.isRight)
}
