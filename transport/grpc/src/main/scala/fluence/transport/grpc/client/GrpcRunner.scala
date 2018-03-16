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

package fluence.transport.grpc.client

import cats.data.EitherT
import cats.effect.{ Async, IO }
import cats.~>

import scala.concurrent.{ ExecutionContext, Future }
import scala.language.{ higherKinds, reflectiveCalls }

object GrpcRunner {
  // Type that converts Future into EitherT, and converts all the known errors on remote grpc call into E
  type Run[F[_], E] = Future ~> ({ type λ[α] = EitherT[F, E, α] })#λ

  implicit def runner[F[_] : Async, E](implicit convert: GrpcError ⇒ E, ctx: ExecutionContext): Run[F, E] =
    new Run[F, E] {
      override def apply[A](fa: Future[A]): EitherT[F, E, A] =
        EitherT(
          IO.fromFuture(IO(fa))
            .attempt
            .map(_.left.map(err ⇒ convert(GrpcError(err))))
            .to[F])
    }
}
