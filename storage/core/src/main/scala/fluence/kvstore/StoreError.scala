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

package fluence.kvstore

import scala.util.control.NoStackTrace

/**
 * Error for any key value store.
 *
 * @param message Error message
 * @param causedBy Error cause for wrapped exceptions
 */
class StoreError(message: String, causedBy: Option[Throwable] = None) extends NoStackTrace {

  override def getMessage: String = message

  override def getCause: Throwable = causedBy getOrElse super.getCause

}

object StoreError {

  def apply(message: String, causedBy: Option[Throwable] = None): StoreError = new StoreError(message, causedBy)

  def apply(exception: Throwable): StoreError = new StoreError(exception.getMessage, Some(exception))

  def getError[K](key: K, causedBy: Option[Throwable] = None): StoreError =
    new StoreError(s"Can't get value for key=$key", causedBy)

  def traverseError(causedBy: Option[Throwable] = None): StoreError =
    new StoreError(s"Traverse failed", causedBy)

  def putError[K, V](key: K, value: V, causedBy: Option[Throwable] = None): StoreError =
    new StoreError(s"Can't put ($key, $value)", causedBy)

  def removeError[K](key: K, causedBy: Option[Throwable] = None): StoreError =
    new StoreError(s"Can't remove key-value pair for key=$key", causedBy)

}
