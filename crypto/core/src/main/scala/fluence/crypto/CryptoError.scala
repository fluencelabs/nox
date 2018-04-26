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

package fluence.crypto

import cats.Applicative
import cats.data.EitherT

import scala.util.control.{NoStackTrace, NonFatal}

case class CryptoError(message: String, causedBy: Option[Throwable] = None) extends NoStackTrace {
  override def getMessage: String = message

  override def getCause: Throwable = causedBy getOrElse super.getCause
}

object CryptoError {

  // TODO: there's a common `catchNonFatal` pattern, we should refactor this metod onto it
  def nonFatalHandling[F[_]: Applicative, A](a: ⇒ A)(errorText: String): EitherT[F, CryptoError, A] =
    try EitherT.pure(a)
    catch {
      case NonFatal(e) ⇒ EitherT.leftT(CryptoError(errorText + ": " + e.getLocalizedMessage, Some(e)))
    }
}
