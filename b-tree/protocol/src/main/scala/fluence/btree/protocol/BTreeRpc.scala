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

import fluence.btree.core.{ClientPutDetails, Hash, Key}
import scodec.bits.ByteVector

import scala.collection.Searching.SearchResult
import scala.language.higherKinds

object BTreeRpc {

  /**
   * Base parent for all callback wrappers needed for any operation to the BTree.
   *
   * @tparam F An effect, with MonadError
   */
  trait BtreeCallback[F[_]] {

    /**
     * Server asks next child node index.
     *
     * @param keys              Keys of current branch for searching index
     * @param childsChecksums  All children checksums of current branch
     */
    def nextChildIndex(keys: Array[Key], childsChecksums: Array[Hash]): F[Int]

  }

  /**
   * Wrapper for all callback needed for search operation to the BTree.
   * Each callback corresponds to operation needed btree for traversing and getting index.
   *
   * @tparam F An effect, with MonadError
   */
  trait SearchCallback[F[_]] extends BtreeCallback[F] {

    /**
     * Server sends founded leaf details.
     *
     * @param keys              Keys of current leaf
     * @param valuesChecksums  Checksums of values for current leaf
     * @return [[scala.collection.Searching.Found]] is key was found,
     *          [[scala.collection.Searching.InsertionPoint]] otherwise
     */
    def submitLeaf(keys: Array[Key], valuesChecksums: Array[Hash]): F[SearchResult]

  }

  /**
   * Wrapper for all callback needed for ''Put'' operation to the BTree.
   * Each callback corresponds to operation needed btree for traversing and putting value.
   *
   * @tparam F An effect, with MonadError
   */
  trait PutCallbacks[F[_]] extends BtreeCallback[F] {

    /**
     * Server sends founded leaf details.
     *
     * @param keys              Keys of current leaf
     * @param valuesChecksums  Checksums of values for current leaf
     */
    def putDetails(keys: Array[Key], valuesChecksums: Array[Hash]): F[ClientPutDetails]

    /**
     * Server sends new merkle root to client for approve made changes.
     *
     * @param serverMerkleRoot New merkle root after putting key/value
     * @param wasSplitting 'True' id server performed tree rebalancing, 'False' otherwise
     * @return signed by client new contract execution state
     */
    def verifyChanges(serverMerkleRoot: Hash, wasSplitting: Boolean): F[ByteVector]

    /**
     * Server confirms that all changes was persisted.
     */
    def changesStored(): F[Unit]

  }

  // not ready yet, maybe should use SearchCallback instead
  trait RemoveCallback[F[_]] extends BtreeCallback[F]

}
