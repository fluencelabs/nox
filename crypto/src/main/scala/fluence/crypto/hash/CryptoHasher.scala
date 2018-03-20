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

package fluence.crypto.hash

import cats.kernel.Semigroup
import cats.{Contravariant, Functor, Monoid, Traverse}

import scala.language.{higherKinds, reflectiveCalls}

/**
 * TODO add F[_] effect
 * Base interface for hashing.
 *
 * @tparam M type of message for hashing
 * @tparam H type of hashed message
 */
trait CryptoHasher[M, H] {
  self ⇒

  def hash(msg: M): H

  def hash(msg1: M, msgN: M*): H

  def hashM[F[_]: Traverse](ms: F[M])(implicit M: Monoid[M]): H =
    hash(Traverse[F].fold(ms))

  final def map[B](f: H ⇒ B): CryptoHasher[M, B] = new CryptoHasher[M, B] {
    override def hash(msg: M): B = f(self.hash(msg))

    override def hash(msg1: M, msgN: M*): B = f(self.hash(msg1, msgN: _*))
  }

  final def contramap[N](f: N ⇒ M): CryptoHasher[N, H] = new CryptoHasher[N, H] {
    override def hash(msg: N): H = self.hash(f(msg))

    override def hash(msg1: N, msgN: N*): H = self.hash(f(msg1), msgN.map(f): _*)
  }

}

object CryptoHasher {
  type Bytes = CryptoHasher[Array[Byte], Array[Byte]]

  def buildM[M: Semigroup, H](h: M ⇒ H): CryptoHasher[M, H] = new CryptoHasher[M, H] {
    override def hash(msg: M): H = h(msg)

    override def hash(msg1: M, msgN: M*): H = h(msgN.foldLeft(msg1)(Semigroup[M].combine))
  }

  implicit def cryptoHasherFunctor[M]: Functor[({ type λ[α] = CryptoHasher[M, α] })#λ] =
    new Functor[({ type λ[α] = CryptoHasher[M, α] })#λ] {
      override def map[A, B](fa: CryptoHasher[M, A])(f: A ⇒ B): CryptoHasher[M, B] =
        fa.map(f)
    }

  implicit def cryptoHasherContravariant[H]: Contravariant[({ type λ[α] = CryptoHasher[α, H] })#λ] =
    new Contravariant[({ type λ[α] = CryptoHasher[α, H] })#λ] {
      override def contramap[A, B](fa: CryptoHasher[A, H])(f: B ⇒ A): CryptoHasher[B, H] =
        fa.contramap(f)
    }
}
