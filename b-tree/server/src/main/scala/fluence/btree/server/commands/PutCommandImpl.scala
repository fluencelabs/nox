/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.btree.server.commands

import cats.MonadError
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.btree.common.ValueRef
import fluence.btree.common.merkle.{MerklePath, MerkleRootCalculator}
import fluence.btree.core.{Hash, Key}
import fluence.btree.protocol.BTreeRpc.PutCallbacks
import fluence.btree.server.core._
import fluence.btree.server.{Leaf, NodeId}

import scala.language.higherKinds

/**
 * Command for putting key and value to the BTree.
 *
 * @param merkleRootCalculator A function that ask client to give some required details for the next step
 * @param putCallbacks          A pack of functions that ask client to give some required details for the next step
 * @param valueRefProvider      A function for getting next value reference
 * @tparam F The type of effect, box for returning value
 */
case class PutCommandImpl[F[_]](
  merkleRootCalculator: MerkleRootCalculator,
  putCallbacks: PutCallbacks[F],
  valueRefProvider: () ⇒ ValueRef
)(implicit ME: MonadError[F, Throwable])
    extends BaseSearchCommand[F](putCallbacks) with PutCommand[F, Key, ValueRef, NodeId] {

  def putDetails(leaf: Option[Leaf]): F[BTreePutDetails] = {
    val (keys, valuesChecksums) =
      leaf
        .map(l ⇒ l.keys → l.valuesChecksums)
        .getOrElse(Array.empty[Key] → Array.empty[Hash])

    putCallbacks
      .putDetails(keys, valuesChecksums)
      .map(clientPutDetails ⇒ BTreePutDetails(clientPutDetails, valueRefProvider))
  }

  override def verifyChanges(merklePath: MerklePath, wasSplitting: Boolean): F[Unit] = {
    putCallbacks.verifyChanges(merkleRootCalculator.calcMerkleRoot(merklePath), wasSplitting).flatMap { _ ⇒
      putCallbacks.changesStored()
    }
  }
}
