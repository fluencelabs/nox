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
import cats.laws.discipline.ArrowChoiceTests
import cats.tests.CatsSuite
import org.scalacheck.{Arbitrary, Cogen, Gen, Prop}
import org.scalacheck.ScalacheckShapeless._

class FuncELawsSpec extends CatsSuite {
  implicit def arbFunc[E <: Throwable: Arbitrary, A: Arbitrary: Cogen, B: Arbitrary]: Arbitrary[FuncE[E, A, B]] =
    Arbitrary(
      Gen
        .function1[A, (Boolean, B, E)](
          Gen.zip(Arbitrary.arbitrary[Boolean], Arbitrary.arbitrary[B], Arbitrary.arbitrary[E])
        )
        .map(_.andThen { case (x, y, z) ⇒ Either.cond(x, y, z) })
        .map(f ⇒ FuncE.liftEither(f))
    )

  implicit def eqFunc[E <: Throwable, A: Arbitrary, B]: Eq[FuncE[E, A, B]] = Eq.instance { (x, y) ⇒
    Prop.forAll { input: A ⇒
      x.apply[Id](input).value == y.apply[Id](input).value
    }.check
    true
  }

  checkAll(
    "FuncE.ArrowChoiceLaws",
    ArrowChoiceTests[FuncE[CodecError, ?, ?]].arrowChoice[Int, String, Double, BigDecimal, Long, Short]
  )
}
