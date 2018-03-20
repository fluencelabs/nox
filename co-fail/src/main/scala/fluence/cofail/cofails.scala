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

package fluence.cofail

import cats.Applicative
import cats.data.EitherT

import scala.language.higherKinds
import scala.util.control.NonFatal

/**
  * This is a common supertype for all application-specific errors.
  * We consider subtyped errors as managed by us, and all others as exceptions.
  *
  * @param message Human-readable message
  */
private[cofail] sealed abstract class CoFail(val message: String, val errorType: String)

/**
  * Error for any kind of input format validation
  * @param message Human-readable message
  */
sealed class InputErr(message: String) extends CoFail(message, "input")

object InputErr {
  def apply(message: String): InputErr = new InputErr(message)

  def apply(reason: Throwable): InputErr = new InputErr(reason.getMessage)
}

/**
  * Network-related errors
  * @param message Human-readable message
  */
sealed class NetworkErr(message: String) extends CoFail(message, "network")

object NetworkErr {
  def unavailable(message: String): NetworkErr = new NetworkErr(message)

  def apply(message: String): NetworkErr = new NetworkErr(message)

  def apply(reason: Throwable): NetworkErr = new NetworkErr(reason.getMessage)
}

/**
  * Errors in crypto, both
  * @param message Human-readable message
  */
sealed class CryptoErr(message: String) extends CoFail(message, "crypto")

object CryptoErr {
  def apply(message: String): CryptoErr = new CryptoErr(message)

  def apply(reason: Throwable): CryptoErr = new CryptoErr(reason.getMessage)

  def catchNonFatal[F[_]: Applicative, A](errorText: ⇒ String, a: ⇒ A): EitherT[F, CryptoErr, A] =
    try EitherT.pure(a)
    catch {
      case NonFatal(e) ⇒ EitherT.leftT(CryptoErr(errorText + " " + e.getLocalizedMessage))
    }
}
