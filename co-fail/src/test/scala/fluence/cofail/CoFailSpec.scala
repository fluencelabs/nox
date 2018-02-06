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

import cats.MonadError
import org.scalatest.{ Matchers, WordSpec }
import shapeless._
import scala.language.higherKinds

class CoFailSpec extends WordSpec with Matchers {

  "co-fail" should {
    "compile" in {
      class CheckCompiles[F[_]](ME: MonadError[F, Throwable]) {

        object Implicits {
          implicit val TME: MonadError[F, Throwable] = ME

          implicit val F: MonadError[F, A :+: B :+: NoSuchElementException :+: CNil] =
            CoFail.fromThrowableME(TME)
        }

        case class A(a: String)
        case class B(b: String)

        def toCoFail: MonadError[F, A :+: B :+: CNil] = {
          import CoFail._
          import Implicits.F

          pickImplicit() // check that it can pick one implicitly
          narrowImplicit() // check that it can narrow down implicitly

          ???
        }

        def pickImplicit()(implicit F: MonadError[F, A]): Unit = ()

        def narrowImplicit()(implicit F: MonadError[F, A :+: NoSuchElementException :+: CNil]): Unit = ()

      }

      1 shouldBe 1
    }
  }

}
