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

package fluence.btree.client.network

import fluence.btree.client.core.PutDetails
import fluence.btree.client.network.BTreeRpc.{ GetCallbacks, PutCallbacks }
import fluence.btree.client.{ Bytes, Key, Value }

// todo docs
trait BTreeRpc[F[_]] {

  def get(callbacks: GetCallbacks[F]): F[Unit]

  def put(callbacks: PutCallbacks[F]): F[Unit]

}

object BTreeRpc {

  trait GetCallbacks[F[_]] {

    // case when server asks next child
    def nextChildIndex(keys: Array[Key], childsChecksums: Array[Bytes]): F[Int]

    // case when server returns founded leaf
    def submitLeaf(keys: Array[Key], values: Array[Value]): F[Unit]

  }

  trait PutCallbacks[F[_]] {

    // case when server asks next child
    def nextChildIndex(keys: Array[Key], childsChecksums: Array[Bytes]): F[Int]

    // case when server returns founded leaf
    def putDetails(keys: Array[Key], values: Array[Value]): F[PutDetails]

    // case when server asks verify made changes
    def verifyChanges(serverMerkleRoot: Bytes, wasSplitting: Boolean): F[Unit]

    // case when server confirmed changes persisted
    def changesStored(): F[Unit]

  }

}
