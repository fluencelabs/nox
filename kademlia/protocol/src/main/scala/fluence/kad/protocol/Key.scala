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

package fluence.kad.protocol

import java.lang.Byte.toUnsignedInt
import java.nio.charset.Charset
import java.security.MessageDigest

import cats.syntax.monoid._
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.{ MonadError, Monoid, Order, Show }
import fluence.codec.Codec
import fluence.crypto.keypair.KeyPair
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.util.Try

/**
 * Kademlia Key is 160 bits (sha-1 length) in byte array.
 * We use value case class for type safety, and typeclasses for ops.
 *
 * @param value ID wrapped with ByteVector
 */
final case class Key private (value: ByteVector) extends AnyVal {
  def id: Array[Byte] = value.toArray

  /**
   * Number of leading zeros
   */
  def zerosPrefixLen: Int = {
    val idx = id.indexWhere(_ != 0)
    if (idx < 0) {
      Key.BitLength
    } else {
      Integer.numberOfLeadingZeros(toUnsignedInt(id(idx))) + java.lang.Byte.SIZE * (idx - 3)
    }
  }

  def b64: String = value.toBase64

  override def toString: String = b64
}

object Key {
  val Length = 20
  val BitLength: Int = Length * 8

  // XOR Monoid is used for Kademlia distance
  implicit object XorDistanceMonoid extends Monoid[Key] {
    override val empty: Key = Key(ByteVector.fill(Length)(0: Byte)) // filled with zeros

    override def combine(x: Key, y: Key): Key = Key(x.value ^ y.value)
  }

  // Kademlia keys are ordered, low order byte is the most significant
  implicit object OrderedKeys extends Order[Key] {
    override def compare(x: Key, y: Key): Int = {
      var i = 0
      while (i < Length) {
        if (x.id(i) != y.id(i)) {
          return toUnsignedInt(x.id(i)) compareTo toUnsignedInt(y.id(i))
        }
        i += 1
      }
      0
    }
  }

  // Order relative to a distinct key
  def relativeOrder(key: Key): Order[Key] =
    (x, y) ⇒ OrderedKeys.compare(x |+| key, y |+| key)

  def relativeOrdering(key: Key): Ordering[Key] =
    relativeOrder(key).compare(_, _)

  implicit object ShowKeyBase64 extends Show[Key] {
    override def show(f: Key): String = f.b64
  }

  /**
   * Tries to read base64 form of Kademlia key.
   */
  def fromB64[F[_]](str: String)(implicit F: MonadError[F, Throwable]): F[Key] =
    b64Codec[F].decode(str)

  /**
   * Checks that given key is produced form that publicKey
   *
   * @param key Kademlia Key, should be sha1 of publicKey
   * @param publicKey Public Key
   * @return
   */
  def checkPublicKey(key: Key, publicKey: KeyPair.Public): Boolean = {
    import cats.instances.try_._
    import cats.syntax.eq._
    sha1[Try](publicKey.value.toArray).filter(_ === key).isSuccess
  }

  /**
   * Calculates sha-1 hash of the payload, and wraps it with Key.
   * TODO: use hasher from crypto
   *
   * @param bytes Bytes to hash
   */
  def sha1[F[_]](bytes: Array[Byte])(implicit F: MonadError[F, Throwable]): F[Key] =
    F.catchNonFatal{
      val md = MessageDigest.getInstance("SHA-1")
      md.digest(bytes)
    }.flatMap(fromBytes[F])

  def fromKeyPair[F[_]](keyPair: KeyPair)(implicit F: MonadError[F, Throwable]): F[Key] =
    fromPublicKey(keyPair.publicKey)

  def fromPublicKey[F[_]](publicKey: KeyPair.Public)(implicit F: MonadError[F, Throwable]): F[Key] =
    sha1(publicKey.value.toArray)

  def fromString[F[_]](str: String, charset: Charset = Charset.defaultCharset())(implicit F: MonadError[F, Throwable]): F[Key] =
    sha1(str.getBytes)

  def fromBytes[F[_]](bytes: Array[Byte])(implicit F: MonadError[F, Throwable]): F[Key] =
    bytesCodec[F].decode(bytes)

  implicit def bytesCodec[F[_]](implicit F: MonadError[F, Throwable]): Codec[F, Key, Array[Byte]] =
    vectorCodec[F] andThen Codec.codec[F, ByteVector, Array[Byte]]

  implicit def b64Codec[F[_]](implicit F: MonadError[F, Throwable]): Codec[F, Key, String] =
    vectorCodec[F] andThen Codec.codec[F, ByteVector, String]

  implicit def vectorCodec[F[_]](implicit F: MonadError[F, Throwable]): Codec[F, Key, ByteVector] =
    Codec(
      _.value.pure[F],
      vec ⇒
        if (vec.size == Length) Key(vec).pure[F]
        else F.raiseError(new IllegalArgumentException(s"Key length in bytes must be $Length, but ${vec.size} given"))
    )
}
