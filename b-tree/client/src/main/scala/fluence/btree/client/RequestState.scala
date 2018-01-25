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

import fluence.btree.common.{ Bytes, Hash }

import scala.language.higherKinds

/**
 * State of any request from client to server.
 */
sealed trait RequestState

/**
 * State for each 'Get' request to remote BTree. One ''GetState'' corresponds to one series of round trip requests
 */
trait GetState[F[_]] extends RequestState {

  /** Returns result of 'get' operation. */
  def getFoundValue: F[Option[Hash]] // todo this should be removed in the next task

}

/**
 * State for each 'Put' request to remote BTree. One ''PutState'' corresponds to one series of round trip requests
 */
trait PutState[F[_]] extends RequestState {

  /** Returns old value that has been stored in tree before current 'put' operation. */
  def getOldValue: F[Option[Bytes]] // todo this should be removed in the next task

}
