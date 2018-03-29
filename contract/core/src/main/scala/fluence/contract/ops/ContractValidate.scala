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

package fluence.contract.ops

import cats.data.EitherT
import cats.{Monad, MonadError}
import fluence.contract.ops.ContractValidate.ValidationErr
import fluence.crypto.SignAlgo.CheckerFn

import scala.language.higherKinds
import scala.util.control.NoStackTrace

/**
 * Contract validator. Try to use it near to Contract codec,
 * after decoding and before encoding for reducing error prone
 *
 * @tparam C A type of contract
 */
trait ContractValidate[C] {

  /**
   * Verifies the correctness of the contract.
   */
  def validate[F[_]: Monad](contract: C)(implicit checkerFn: CheckerFn): EitherT[F, ValidationErr, Unit]

  /**
   * Verifies the correctness of the contract. Do the same as [[ContractValidate.validate]],
   * but return MonadError instead EitherT.
   *
   * Todo: will be removed in future, used only as temporary adapter
   */
  @deprecated("Try to use validate with EitherT instead if it's possible")
  def validateME[F[_]](contract: C)(implicit ME: MonadError[F, Throwable], checkerFn: CheckerFn): F[Unit]

}

object ContractValidate {

  case class ValidationErr(msg: String, cause: Throwable) extends NoStackTrace {
    override def getMessage: String = msg
    override def getCause: Throwable = cause
  }

  implicit class ContractValidatorOps[C](contract: C)(implicit cValidate: ContractValidate[C], checkerFn: CheckerFn) {

    def validate[F[_]: Monad]: EitherT[F, ValidationErr, Unit] =
      cValidate.validate(contract)

    @deprecated("Try to use validate with EitherT instead if it's possible")
    def validateME[F[_]](implicit ME: MonadError[F, Throwable]): F[Unit] =
      cValidate.validateME(contract)

  }

}
