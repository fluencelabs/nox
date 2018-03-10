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

package fluence.btree.client

import fluence.btree.core.Hash

import scala.language.higherKinds

/**
 * Remote MerkleBTree api.
 *
 * @tparam F A box for returning value
 * @tparam K The type of plain text keys
 */
trait MerkleBTreeClientApi[F[_], K] {

  /**
   * Returns ''GetState'' with callbacks for finding ''value'' for specified ''key'' in remote MerkleBTree.
   *
   * @param key Plain text key
   */
  def initGet(key: K): F[GetState[F]]

  /**
   * Returns ''PutState'' with callbacks for saving encrypted ''key'' and ''value'' into remote MerkleBTree.
   *
   * @param key             Plain text key
   * @param valueChecksum  Checksum of encrypted value to be store
   */
  def initPut(key: K, valueChecksum: Hash): F[PutState[F]]

  /**
   * Returns ''RemoveState'' with callbacks for deleting ''key value pair'' into remote MerkleBTree by
   * specifying plain text key.
   *
   * @param key Plain text key
   */
  def removeState(key: K): F[RemoveState[F]]

}
