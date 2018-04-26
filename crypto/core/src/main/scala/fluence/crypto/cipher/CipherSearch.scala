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

package fluence.crypto.cipher

import cats.Monad
import cats.data.EitherT
import fluence.crypto.{Crypto, CryptoError}

import scala.collection.Searching.{Found, InsertionPoint, SearchResult}
import scala.language.higherKinds

object CipherSearch {

  def binary[A, B](coll: IndexedSeq[A], decrypt: Crypto.Func[A, B])(
    implicit ordering: Ordering[B]
  ): Crypto.Func[B, SearchResult] =
    new Crypto.Func[B, SearchResult] {
      override def apply[F[_]](input: B)(
        implicit F: Monad[F]
      ): EitherT[F, CryptoError, SearchResult] = {
        type M[X] = EitherT[F, CryptoError, X]
        implicitly[Monad[M]].tailRecM((0, coll.length)) {
          case (from, to) if from == to ⇒ EitherT.rightT(Right(InsertionPoint(from)))
          case (from, to) ⇒
            val idx = from + (to - from - 1) / 2
            decrypt(coll(idx)).map { d ⇒
              math.signum(ordering.compare(input, d)) match {
                case -1 ⇒ Left((from, idx))
                case 1 ⇒ Left((idx + 1, to))
                case _ ⇒ Right(Found(idx))
              }
            }
        }
      }
    }
}
