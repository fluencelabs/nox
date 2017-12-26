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
import fluence.btree.client.core.PutDetails
import fluence.btree.client.merkle.{ MerklePath, MerkleRootCalculator }
import fluence.btree.client.protocol.BTreeRpc.PutCallbacks
import fluence.btree.client.{ Key, Value }
import fluence.btree.server.Leaf
import fluence.btree.server.core._
import cats.syntax.flatMap._

/**
 * Command for putting key and value to the BTree.
 *
 * @param merkleRootCalculator A function that ask client to give some required details for the next step
 * @param putCallbacks          A pack of functions that ask client to give some required details for the next step
 * @tparam F The type of effect, box for returning value
 */
case class PutCommandImpl[F[_]](
    merkleRootCalculator: MerkleRootCalculator,
    putCallbacks: PutCallbacks[F]
)(implicit ME: MonadError[F, Throwable]) extends BaseSearchCommand[F](putCallbacks) with PutCommand[F, Key, Value] {

  def putDetails(leaf: Option[Leaf]): F[PutDetails] = {
    val (keys, values) =
      leaf.map(l ⇒ l.keys → l.values)
        .getOrElse(Array.empty[Key] → Array.empty[Value])

    putCallbacks.putDetails(keys, values)
  }

  override def verifyChanges(merklePath: MerklePath, wasSplitting: Boolean): F[Unit] = {
    putCallbacks.verifyChanges(merkleRootCalculator.calcMerkleRoot(merklePath), wasSplitting)
      .flatMap { _ ⇒ putCallbacks.changesStored() }
  }
}
