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

import cats.effect.Clock
import cats.{Applicative, Monad, Parallel, Traverse}
import cats.instances.list._
import cats.syntax.applicative._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.monoid._
import cats.syntax.order._
import cats.syntax.semigroupk._
import fluence.kad.contact.ContactAccess
import fluence.kad.protocol.{Key, Node}
import fluence.log.Log

import scala.annotation.tailrec
import scala.language.higherKinds

private[state] class RoutingStateImpl[F[_]: Monad, P[_], C](
  override val nodeKey: Key,
  siblingsState: SiblingsState[F, C],
  bucketsState: BucketsState[F, C]
)(implicit P: Parallel[F])
    extends RoutingState[F, C] {

  override val siblings: F[Siblings[C]] =
    siblingsState.read

  override def bucket(distanceKey: Key): F[Bucket[C]] =
    bucketsState.read(distanceKey)

  override def bucket(bucketId: Int): F[Bucket[C]] =
    bucketsState.read(bucketId)

  /**
   * Removes a node from routing table by its key, returns optional removed node
   *
   * @param key Key
   * @return Optional node, if it was removed
   */
  override def remove(key: Key)(implicit log: Log[F]): F[ModResult[C]] =
    P sequential P.apply.map2(
      P parallel siblingsState.remove(key),
      P parallel bucketsState.remove(key distanceTo nodeKey, key)
    )(_ <+> _)

  /**
   * Locates the bucket responsible for given contact, and updates it using given ping function
   *
   * @param node          Contact to update
   * @return ModResult
   */
  override def update(
    node: Node[C]
  )(implicit clock: Clock[F], ca: ContactAccess[F, C], log: Log[F]): F[ModResult[C]] =
    updateWithKeepExisting(node, keepExisting = true)

  /**
   * Implementation for [[update]] with managed ModResult checks -- required for [[updateList]] optimization
   */
  private def updateWithKeepExisting(
    node: Node[C],
    keepExisting: Boolean
  )(implicit clock: Clock[F], ca: ContactAccess[F, C], log: Log[F]): F[ModResult[C]] =
    if (nodeKey === node.key)
      ModResult.noop[C].pure[F]
    else
      ca.check(node).flatMap {
        case true ⇒
          log.trace(s"Update node: ${node.key}") *>
            P.sequential(
                P.apply.map2(
                  // Update bucket, performing ping if necessary
                  P parallel bucketsState.update(node.key distanceTo nodeKey, node, ca.rpc, ca.pingExpiresIn),
                  // Update siblings
                  P parallel siblingsState.add(node)
                )(_ <+> _)
              )
              .flatMap(if (keepExisting) keepExistingNodes else _.pure[F])

        case false ⇒
          log.trace(s"Node check failed $node") as
            ModResult.noop[C]
      }

  /**
   * Update RoutingTable with a list of fresh nodes
   *
   * @param nodes         List of new nodes
   * @return Result of state modification
   */
  override def updateList(
    nodes: List[Node[C]]
  )(implicit clock: Clock[F], ca: ContactAccess[F, C], log: Log[F]): F[ModResult[C]] = {
    // From iterable of groups, make list of list of items from different groups
    @tailrec
    def rearrange(groups: Iterable[List[Node[C]]], agg: List[List[Node[C]]] = Nil): List[List[Node[C]]] =
      if (groups.isEmpty) agg
      else {
        val (current, next) = groups.collect { case head :: tail => head -> tail }.unzip
        rearrange(next.toList, current.toList :: agg)
      }

    // Update portion, taking nodes one by one, and return all updated nodes
    def updatePortion(portion: List[Node[C]], agg: ModResult[C] = ModResult.noop): F[ModResult[C]] =
      portion match {
        case Nil ⇒ agg.pure[F]
        case node :: tail ⇒
          updateWithKeepExisting(node, keepExisting = false).flatMap { res ⇒
            updatePortion(tail, res <+> agg)
          }
      }

    // Update each portion in parallel, and return all updated nodes
    def updateParPortions(portions: List[List[Node[C]]]): F[ModResult[C]] =
      Parallel.parTraverse(portions)(updatePortion(_)).map(_.foldLeft(ModResult.noop[C])(_ <+> _))

    updateParPortions(
      // Rearrange in portions with distinct bucket ids, so that it's possible to update it in parallel
      rearrange(
        // Group by bucketId, so that each group should never be updated in parallel
        nodes.groupBy(_.key distanceTo nodeKey).values
      )
    ).flatMap(keepExistingNodes)
  }

  /**
   * Keep existing nodes in case they were dropped either from buckets or siblings, but kept in another storage.
   *
   * @param res Mod result
   * @return ModResult
   */
  private def keepExistingNodes(res: ModResult[C])(implicit log: Log[F]): F[ModResult[C]] =
    Traverse[List]
      .traverse(res.removed.toList)(
        k ⇒
          // TODO is it optimal?
          (
            siblingsState.read.map(_.find(k)),
            bucketsState.read(nodeKey |+| k).map(_.find(k))
          ).mapN(_ orElse _)
      )
      .flatMap(
        nodes ⇒
          Monad[F].tailRecM(nodes.flatten -> res) {
            case (Nil, mr) ⇒
              Applicative[F].pure(Right(mr))
            case (n :: tail, mr) ⇒
              log.trace(s"Kept ${n.key}, as it's still present in routing table") as Left(tail -> mr.keep(n.key))
          }
      )
}
