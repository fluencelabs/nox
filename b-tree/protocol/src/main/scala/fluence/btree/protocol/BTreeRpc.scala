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

package fluence.btree.protocol

import fluence.btree.common.{ Bytes, Key, PutDetails, Value }
import fluence.btree.protocol.BTreeRpc.{ GetCallbacks, PutCallbacks }

import scala.language.higherKinds

/**
 * An interface to calls for a remote BTree server.
 *
 * @tparam F An effect, with MonadError
 */
trait BTreeRpc[F[_]] {

  /**
   * Initiates ''Get'' operation in remote MerkleBTree.
   *
   * @param callbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   */
  def get(callbacks: GetCallbacks[F]): F[Unit]

  /**
   * Initiates ''Put'' operation in remote MerkleBTree.
   *
   * @param callbacks Wrapper for all callback needed for ''Put'' operation to the BTree.
   */
  def put(callbacks: PutCallbacks[F]): F[Unit]

}

object BTreeRpc {

  /**
   * Base parent for all callback wrappers needed for any operation to the BTree.
   *
   * @tparam F An effect, with MonadError
   */
  trait SearchCallback[F[_]] {

    /**
     * Server asks next child node index.
     *
     * @param keys              Keys of current branch for searching index
     * @param childsChecksums  All children checksums of current branch
     */
    def nextChildIndex(keys: Array[Key], childsChecksums: Array[Bytes]): F[Int]

  }

  /**
   * Wrapper for all callback needed for ''Get'' operation to the BTree.
   * Each callback corresponds to operation needed btree for traversing and getting value.
   *
   * @tparam F An effect, with MonadError
   */
  trait GetCallbacks[F[_]] extends SearchCallback[F] {

    /**
     * Server sends founded leaf details.
     *
     * @param keys    Keys of current leaf
     * @param values  Values of current leaf
     */
    def submitLeaf(keys: Array[Key], values: Array[Value]): F[Unit]

  }

  /**
   * Wrapper for all callback needed for ''Put'' operation to the BTree.
   * Each callback corresponds to operation needed btree for traversing and putting value.
   *
   * @tparam F An effect, with MonadError
   */
  trait PutCallbacks[F[_]] extends SearchCallback[F] {

    /**
     * Server sends founded leaf details.
     *
     * @param keys    Keys of current leaf
     * @param values  Values of current leaf
     */
    def putDetails(keys: Array[Key], values: Array[Value]): F[PutDetails]

    /**
     * Server sends new merkle root to client for approve made changes.
     *
     * @param serverMerkleRoot New merkle root after putting key/value
     * @param wasSplitting      'True' id server performed tree rebalancing, 'False' otherwise
     */
    def verifyChanges(serverMerkleRoot: Bytes, wasSplitting: Boolean): F[Unit]

    /**
     * Server confirms that all changes was persisted.
     */
    def changesStored(): F[Unit]

  }

}
