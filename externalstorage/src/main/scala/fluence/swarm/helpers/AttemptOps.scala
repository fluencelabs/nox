/*
 * Copyright (C) 2018  Fluence Labs Limited
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

package fluence.swarm.helpers
import cats.Applicative
import cats.data.EitherT
import scodec.{Attempt, Err}

import scala.language.higherKinds

object AttemptOps {
  implicit class RichAttempt[T](attempt: Attempt[T]) {
    def toEitherT[F[_]: Applicative]: EitherT[F, Err, T] = EitherT.fromEither(attempt.toEither)
    def toEitherT[F[_]: Applicative, E](errFunc: Err => E): EitherT[F, E, T] = toEitherT.leftMap(errFunc)
  }
}
