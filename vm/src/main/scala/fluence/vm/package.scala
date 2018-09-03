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

package fluence
import cats.Applicative
import cats.data.EitherT

import scala.language.higherKinds

package object vm {

  /** Helper method. Converts List of Ether to Either of List. */
  def list2Either[F[_]: Applicative, A, B](
    list: List[Either[A, B]]
  ): EitherT[F, A, List[B]] = {
    import cats.instances.list._
    import cats.syntax.traverse._
    import cats.instances.either._
    // unfortunately Idea don't understand this and show error in Editor
    val either: Either[A, List[B]] = list.sequence
    EitherT.fromEither[F](either)
  }

}
