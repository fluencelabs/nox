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

import fluence.btree.client.{ Key, Value }
import scala.collection.Searching.SearchResult

/** Parent type for any requests from client to server. */
sealed trait BTreeClientRequest

/** Parent type for initial requests from client to server. */
sealed trait InitClientRequest extends BTreeClientRequest

/** Initial request for start getting value from server BTree. */
object InitGetRequest extends InitClientRequest

/** Initial request for start putting value from server BTree. */
object InitPutRequest extends InitClientRequest

/**
 * Request for resuming descent down the tree.
 *
 * @param nextChildIdx Next child node index, for making next step down the tree.
 */
case class ResumeSearchRequest(nextChildIdx: Int) extends BTreeClientRequest

/**
 * Request for putting new key and value into tree.
 *
 * @param key           Encrypted key
 * @param value         Encrypted value
 * @param searchResult Result of searching client key in server leaf keys. Contains an index
 *                       for putting specified key and value
 */
case class PutRequest(key: Key, value: Value, searchResult: SearchResult) extends BTreeClientRequest

/** Parent type for any final client request for any mutation operation into the tree. */
sealed trait FinishClientRequest extends BTreeClientRequest

/** Confirmation for last response from server BTree. The client accepted the server response */
object Confirm extends FinishClientRequest

/** Rejection for last response from server BTree. The client did not accept the server response */
object Reject extends FinishClientRequest
