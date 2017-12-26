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
import java.nio.ByteBuffer
import java.nio.charset.Charset
import java.security.MessageDigest
import java.util.Base64

import cats.syntax.monoid._
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.{ ApplicativeError, MonadError, Monoid, Order, Show }
import fluence.codec.Codec

import scala.language.higherKinds

/**
 * Kademlia Key is 160 bits (sha-1 length) in byte array.
 * We use value case class for type safety, and typeclasses for ops.
 *
 * @param origin ID wrapped with ByteBuffer
 */
final case class Key private (origin: ByteBuffer) extends AnyVal {
  def id: Array[Byte] = origin.array()

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

  def b64: String = Base64.getEncoder.encodeToString(id)

  override def toString: String = b64
}

object Key {
  val Length = 20
  val BitLength: Int = Length * 8

  // TODO should we wrap implicits with Implicits object?

  // XOR Monoid is used for Kademlia distance
  implicit object XorDistanceMonoid extends Monoid[Key] {
    override val empty: Key = Key(ByteBuffer.wrap(Array.ofDim[Byte](Length))) // filled with zeros

    override def combine(x: Key, y: Key): Key = Key {
      var i = 0
      val ret = Array.ofDim[Byte](Length)
      while (i < Length) {
        ret(i) = (x.id(i) ^ y.id(i)).toByte
        i += 1
      }
      ByteBuffer.wrap(ret)
    }
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
    F.catchNonFatal(Base64.getDecoder.decode(str))
      .flatMap(fromBytes[F])

  /**
   * Calculates sha-1 hash of the payload, and wraps it with Key.
   *
   * @param bytes Bytes to hash
   */
  def sha1[F[_]](bytes: Array[Byte])(implicit F: MonadError[F, Throwable]): F[Key] =
    F.catchNonFatal{
      val md = MessageDigest.getInstance("SHA-1")
      md.digest(bytes)
    }.flatMap(fromBytes[F])

  def fromString[F[_]](str: String, charset: Charset = Charset.defaultCharset())(implicit F: MonadError[F, Throwable]): F[Key] =
    sha1(str.getBytes)

  def fromBuffer[F[_]](bb: ByteBuffer)(implicit F: ApplicativeError[F, Throwable]): F[Key] =
    F.catchNonFatal{
      require(bb.array().length == Length, s"Key's length should be $Length bytes")
      new Key(bb)
    }

  def fromBytes[F[_]](bytes: Array[Byte])(implicit F: ApplicativeError[F, Throwable]): F[Key] =
    fromBuffer(ByteBuffer.wrap(bytes))

  implicit def bytesCodec[F[_]](implicit F: ApplicativeError[F, Throwable]): Codec[F, Key, Array[Byte]] =
    Codec(_.id.pure[F], fromBytes[F])

  implicit def b64Codec[F[_]](implicit F: MonadError[F, Throwable]): Codec[F, Key, String] =
    Codec(_.b64.pure[F], fromB64[F])

  implicit def bufferCodec[F[_]](implicit F: ApplicativeError[F, Throwable]): Codec[F, Key, ByteBuffer] =
    Codec(_.origin.pure[F], fromBuffer[F])

}
