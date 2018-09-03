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
import cats.syntax.functor._
import com.softwaremill.sttp.Response

import scala.language.higherKinds

object ResponseOps {
  implicit class RichResponse[F[_]: Applicative, T](resp: F[Response[T]]) {
    def toEitherT: EitherT[F, String, T] = EitherT(resp.map(_.body))
    def toEitherT[E](errFunc: String => E): EitherT[F, E, T] = toEitherT.leftMap(errFunc)
  }
}
