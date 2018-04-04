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

package fluence.codec

import cats.{Eq, Id}
import cats.laws.discipline.eq._
import org.scalacheck.{Arbitrary, Cogen, Gen}
import org.scalacheck.ScalacheckShapeless._

object PureCodecFuncTestInstances {
  implicit def arbFunc[A: Arbitrary: Cogen, B: Arbitrary]: Arbitrary[PureCodec.Func[A, B]] =
    Arbitrary(
      Gen
        .function1[A, (Boolean, B, PureCodec.Error)](
          Gen.zip(Arbitrary.arbitrary[Boolean], Arbitrary.arbitrary[B], Arbitrary.arbitrary[PureCodec.Error])
        )
        .map(_.andThen { case (x, y, z) ⇒ Either.cond(x, y, z) })
        .map(f ⇒ PureCodec.liftFuncEither(f))
    )

  implicit def eqFunc[A: Arbitrary, B: Eq]: Eq[PureCodec.Func[A, B]] = {
    implicit val eitherEq: Eq[Either[PureCodec.Error, B]] =
      Eq.instance((x, y) ⇒ x.fold(y.left.toOption.contains, y.contains))
    val fnEq = implicitly[Eq[A ⇒ Either[PureCodec.Error, B]]]
    Eq.instance { (x, y) ⇒
      fnEq.eqv(x.apply[Id](_).value, y.apply[Id](_).value)
    }
  }
}
