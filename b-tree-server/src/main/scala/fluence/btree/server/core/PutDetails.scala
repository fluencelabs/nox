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

package fluence.btree.server.core

import fluence.btree.client.{ Key, Value }

import scala.collection.Searching.SearchResult

/**
 * Structure for holding all details needed for putting key and value to BTree.
 *
 * @param key          The key that will be placed to the BTree
 * @param value        The value that will be placed to the BTree
 * @param searchResult A result of searching client key in server leaf keys. Contains an index
 *                     for putting specified key and value
 */
case class PutDetails(key: Key, value: Value, searchResult: SearchResult)
