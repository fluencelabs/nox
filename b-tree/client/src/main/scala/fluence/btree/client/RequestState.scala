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

import fluence.btree.protocol.BTreeRpc.{ GetCallbacks, PutCallbacks, RemoveCallback }
import monix.eval.Task

import scala.language.higherKinds

/**
 * State of any request from client to server.
 */
sealed trait RequestState {

  /**
   * Returns the state into a consistent state.
   */
  def recoverState(): Task[Unit]

}

/**
 * State for each 'Get' request to remote BTree. One ''GetState'' corresponds to one series of round trip requests.
 */
trait GetState[F[_]] extends RequestState with GetCallbacks[F]

/**
 * State for each 'Put' request to remote BTree. One ''PutState'' corresponds to one series of round trip requests.
 */
trait PutState[F[_]] extends RequestState with PutCallbacks[F]

/**
 * State for each 'Remove' request to remote BTree. One ''RemoveState'' corresponds to one series of round trip requests.
 */
trait RemoveState[F[_]] extends RequestState with RemoveCallback[F]
