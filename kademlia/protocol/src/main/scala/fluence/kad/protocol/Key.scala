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

import cats.syntax.monoid._
import cats.syntax.profunctor._
import cats.syntax.compose._
import cats.syntax.eq._
import cats.{Id, Monoid, Order, Show}
import fluence.codec.{CodecError, PureCodec}
import fluence.crypto.hash.CryptoHashers
import fluence.crypto.keypair.KeyPair
import scodec.bits.{BitVector, ByteVector}

import scala.util.Try

/**
 * Kademlia Key is 160 bits (sha-1 length) in byte array.
 * We use value case class for type safety, and typeclasses for ops.
 *
 * @param value ID wrapped with ByteVector
 */
final case class Key private (value: ByteVector) {
  def id: Array[Byte] = value.toArray

  lazy val bits: BitVector = value.toBitVector.padLeft(Key.BitLength)

  /**
   * Number of leading zeros
   */
  lazy val zerosPrefixLen: Int =
    bits.toIndexedSeq.takeWhile(!_).size

  lazy val b64: String = value.toBase64

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
      val xBits = x.bits
      val yBits = y.bits

      var i = 0
      while (i < BitLength) {
        if (xBits(i) != yBits(i)) {
          return xBits(i) compareTo yBits(i)
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

  implicit val keyVectorCodec: PureCodec[Key, ByteVector] =
    PureCodec.liftEitherB(
      k ⇒ Right(k.value),
      vec ⇒
        if (vec.size == Length) Right(Key(vec))
        else Left(CodecError(s"Key length in bytes must be $Length, but ${vec.size} given"))
    )

  implicit val b64Codec: PureCodec[Key, String] =
    keyVectorCodec andThen implicitly[PureCodec[ByteVector, String]]

  implicit val bytesCodec: PureCodec[Key, Array[Byte]] =
    keyVectorCodec andThen implicitly[PureCodec[ByteVector, Array[Byte]]]

  val fromBytes: PureCodec.Func[Array[Byte], Key] =
    bytesCodec.inverse

  /**
   * Tries to read base64 form of Kademlia key.
   */
  val fromB64: PureCodec.Func[String, Key] =
    b64Codec.inverse

  /**
   * Calculates sha-1 hash of the payload, and wraps it with Key.
   * We keep using sha-1 instead of sha-2, because randomness is provided with keypair generation, not hash function.
   */
  val sha1: PureCodec.Func[Array[Byte], Key] = {
    // TODO: it should come from crypto
    PureCodec.liftFuncEither[Array[Byte], Array[Byte]](
      bytes ⇒
        Try(CryptoHashers.Sha1.hash(bytes)).toEither.left
          .map(t ⇒ CodecError("Can't calculate sha1 to produce a Kademlia Key", Some(t)))
    ) andThen fromBytes
  }

  val fromStringSha1: PureCodec.Func[String, Key] =
    sha1.lmap[String](_.getBytes)

  val fromPublicKey: PureCodec.Func[KeyPair.Public, Key] =
    sha1 compose PureCodec.liftFunc(_.value.toArray)

  val fromKeyPair: PureCodec.Func[KeyPair, Key] =
    fromPublicKey.lmap[KeyPair](_.publicKey)

  /**
   * Checks that given key is produced form that publicKey
   *
   * @param key Kademlia Key, should be sha1 of publicKey
   * @param publicKey Public Key
   * @return
   */
  def checkPublicKey(key: Key, publicKey: KeyPair.Public): Boolean =
    sha1[Id](publicKey.value.toArray).value.toOption.exists(_ === key)

}
