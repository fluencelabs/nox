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

import cats.effect.{Clock, LiftIO}
import cats.{Monad, Parallel, Traverse}
import cats.instances.list._
import cats.syntax.applicative._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.monoid._
import cats.syntax.order._
import cats.syntax.semigroupk._
import fluence.kad.protocol.{ContactAccess, Key, Node}

import scala.annotation.tailrec
import scala.collection.mutable.ListBuffer
import scala.language.higherKinds

private[state] class RoutingStateImpl[F[_]: Monad, P[_], C](
  override val nodeKey: Key,
  siblingsState: SiblingsState[F, C],
  bucketsState: BucketsState[F, C]
)(
  implicit P: Parallel[F, P]
) extends RoutingState[F, C] with slogging.LazyLogging {

  override val siblings: F[Siblings[C]] =
    siblingsState.read

  override def bucket(distanceKey: Key): F[Bucket[C]] =
    bucketsState.read(distanceKey)

  override def bucket(idx: Int): F[Bucket[C]] =
    bucketsState.read(idx)

  /**
   * Removes a node from routing table by its key, returns optional removed node
   *
   * @param key Key
   * @return Optional node, if it was removed
   */
  override def remove(key: Key): F[ModResult[C]] =
    P sequential P.apply.map2(
      P parallel siblingsState.remove(key),
      P parallel bucketsState.remove((key |+| nodeKey).zerosPrefixLen, key)
    )(_ <+> _)

  /**
   * Locates the bucket responsible for given contact, and updates it using given ping function
   *
   * @param node          Contact to update
   * @return ModResult
   */
  override def update(
    node: Node[C]
  )(implicit clock: Clock[F], liftIO: LiftIO[F], ca: ContactAccess[C]): F[ModResult[C]] =
    updateWithKeepExisting(node, keepExisting = true)

  /**
   * Implementation for [[update]] with managed ModResult checks -- required for [[updateList]] optimization
   */
  private def updateWithKeepExisting(
    node: Node[C],
    keepExisting: Boolean
  )(implicit clock: Clock[F], liftIO: LiftIO[F], ca: ContactAccess[C]): F[ModResult[C]] =
    if (nodeKey === node.key)
      ModResult.noop[C].pure[F]
    else
      ca.check(node).attempt.to[F].flatMap {
        case Right(true) ⇒
          logger.trace("Update node: {}", node.key)

          P.sequential(
              P.apply.map2(
                // Update bucket, performing ping if necessary
                P parallel bucketsState.update((node.key |+| nodeKey).zerosPrefixLen, node, ca.rpc, ca.pingExpiresIn),
                // Update siblings
                P parallel siblingsState.add(node)
              )(_ <+> _)
            )
            .flatMap(if (keepExisting) keepExistingNodes else _.pure[F])

        case Left(err) ⇒
          logger.trace(s"Node check failed with an exception for $node", err)
          ModResult.noop[C].pure[F]

        case _ ⇒
          ModResult.noop[C].pure[F]
      }

  /**
   * Update RoutingTable with a list of fresh nodes
   *
   * @param nodes         List of new nodes
   * @return Result of state modification
   */
  override def updateList(
    nodes: List[Node[C]]
  )(implicit clock: Clock[F], liftIO: LiftIO[F], ca: ContactAccess[C]): F[ModResult[C]] = {
    // From iterable of groups, make list of list of items from different groups
    @tailrec
    def rearrange(groups: Iterable[List[Node[C]]], agg: List[List[Node[C]]] = Nil): List[List[Node[C]]] = {
      if (groups.isEmpty) agg
      else {
        val current = ListBuffer[Node[C]]()
        val next = ListBuffer[List[Node[C]]]()
        groups.foreach {
          case head :: Nil ⇒
            current.append(head)
          case head :: tail ⇒
            current.append(head)
            next.append(tail)
          case _ ⇒
        }
        rearrange(next.toList, current.toList :: agg)
      }
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
        nodes.groupBy(p ⇒ (p.key |+| nodeKey).zerosPrefixLen).values
      )
    ).flatMap(keepExistingNodes)
  }

  /**
   * Keep existing nodes in case they were dropped either from buckets or siblings, but kept in another storage.
   *
   * @param res Mod result
   * @return ModResult
   */
  private def keepExistingNodes(res: ModResult[C]): F[ModResult[C]] =
    Traverse[List]
      .traverse(res.removed.toList)(
        k ⇒
          // TODO is it optimal?
          (
            siblingsState.read.map(_.find(k)),
            bucketsState.read(nodeKey |+| k).map(_.find(k))
          ).mapN(_ orElse _)
      )
      .map(_.foldLeft(res) {
        case (mr, Some(n)) ⇒ mr.keep(n.key, s"Kept ${n.key}, as it's still present in routing table")
        case (mr, _) ⇒ mr
      })
}
