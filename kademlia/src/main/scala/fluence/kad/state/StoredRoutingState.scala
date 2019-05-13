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

package fluence.kad.state

import cats.Traverse
import cats.effect.{Clock, Concurrent, LiftIO}
import cats.instances.list._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.kvstore.KVStore
import fluence.kad.protocol.{ContactAccess, Key, Node}

import scala.language.higherKinds

/**
 * Wraps a [[RoutingState]] with storage logic: reflects all the [[Node]]'s updates and deletes in the given [[KVStore]]
 *
 * @param routingState RoutingState to delegate to
 * @param store Persistent store for the routing data
 * @tparam F Effect
 * @tparam C Contact
 */
private[state] class StoredRoutingState[F[_]: Concurrent, C](
  routingState: RoutingState[F, C],
  store: KVStore[F, Key, Node[C]]
) extends RoutingState[F, C] {

  /**
   * Takes [[ModResult]] from a state-mutating operation, and passes all the changes to the [[store]]
   *
   * @param res State Modifications result
   * @return Runs on background, always returns just Unit
   */
  private def modResult(res: ModResult[C]): F[Unit] =
    Concurrent[F]
      .start(
        // Put (upsert, insert or update) refreshed nodes
        Traverse[List].traverse(res.updated.toList) {
          case (k, v) ⇒ store.put(k, v).value // TODO: at least log errors?
        } *> // And remove
          Traverse[List].traverse(res.removed.toList)(store.remove(_).value)
      )
      .void

  override def nodeKey: Key = routingState.nodeKey

  override val siblings: F[Siblings[C]] =
    routingState.siblings

  override def bucket(distanceKey: Key): F[Bucket[C]] =
    routingState.bucket(distanceKey)

  override def bucket(idx: Int): F[Bucket[C]] =
    routingState.bucket(idx)

  override def remove(key: Key): F[ModResult[C]] =
    routingState.remove(key).flatTap(modResult)

  override def update(
    node: Node[C]
  )(implicit clock: Clock[F], liftIO: LiftIO[F], ca: ContactAccess[C]): F[ModResult[C]] =
    routingState.update(node).flatTap(modResult)

  override def updateList(
    nodes: List[Node[C]]
  )(implicit clock: Clock[F], liftIO: LiftIO[F], ca: ContactAccess[C]): F[ModResult[C]] =
    routingState.updateList(nodes).flatTap(modResult)

}
