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

package fluence.dataset.protocol.storage

import fluence.btree.protocol.BTreeRpc.{ GetCallbacks, PutCallbacks, RemoveCallback }

import scala.language.higherKinds

/**
 * Remotely-accessible interface to value storage. All parts of storage(btree index, value storage) use this Rpc.
 *
 * @tparam F A box for returning value
 */
trait DatasetStorageRpc[F[_]] {

  /**
   * Initiates ''Get'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param getCallbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   * @return returns found value, None if nothing was found.
   */
  def get(datasetId: Array[Byte], getCallbacks: GetCallbacks[F]): F[Option[Array[Byte]]]

  /**
   * Initiates ''Put'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param putCallbacks     Wrapper for all callback needed for ''Put'' operation to the BTree.
   * @param encryptedValue  Encrypted value.
   * @return returns old value if old value was overridden, None otherwise.
   */
  def put(datasetId: Array[Byte], putCallbacks: PutCallbacks[F], encryptedValue: Array[Byte]): F[Option[Array[Byte]]]

  /**
   * Initiates ''Remove'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param removeCallbacks Wrapper for all callback needed for ''Remove'' operation to the BTree.
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  def remove(datasetId: Array[Byte], removeCallbacks: RemoveCallback[F]): F[Option[Array[Byte]]]

}
