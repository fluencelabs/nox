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

package fluence.crypto.algorithm

import cats.MonadError

import scala.language.higherKinds
import scala.util.control.{ NoStackTrace, NonFatal }

case class CryptoErr(errorMessage: String) extends Throwable(errorMessage) with NoStackTrace

object CryptoErr {
  def nonFatalHandling[F[_], A](a: ⇒ A)(errorText: String)(implicit F: MonadError[F, Throwable]): F[A] = {
    try F.pure(a)
    catch {
      case NonFatal(e) ⇒ F.raiseError(CryptoErr(errorText + " " + e.getLocalizedMessage))
    }
  }
}
