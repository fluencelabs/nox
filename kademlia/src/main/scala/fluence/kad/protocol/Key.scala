/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.kad.protocol

import cats.data.EitherT
import cats.syntax.monoid._
import cats.syntax.profunctor._
import cats.syntax.compose._
import cats.syntax.eq._
import cats.{Id, Monad, Monoid, Order, Show}
import fluence.codec.{CodecError, PureCodec}
import fluence.crypto.hash.CryptoHashers
import scodec.bits.{BitVector, ByteVector}
import fluence.codec.bits.BitsCodecs._
import fluence.codec.bits.BitsCodecs.Base58._
import fluence.crypto.{CryptoError, KeyPair}

import scala.language.higherKinds
import scala.util.Random

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

  lazy val asBase58: String = value.toBase58

  override def toString: String = asBase58

  /**
   * Provides a Key with the same prefix of ''keepNumBits'' size and random suffix.
   *
   * @param distance Size of this key's prefix to keep
   * @param rnd Random instance
   * @return Randomized key
   */
  def randomDistantKey(distance: Int, rnd: Random = Random): Key =
    Key(Range(distance, Key.BitLength).foldLeft(bits)(_.update(_, rnd.nextBoolean())).toByteVector)

  /**
   * Kademlia distance from this key to some other one.
   *
   * @param otherKey Key to calculate distance to
   * @return Distance, could be used as bucketId
   */
  def distanceTo(otherKey: Key): Int =
    Key.XorDistanceMonoid.combine(this, otherKey).zerosPrefixLen
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

  implicit object ShowKeyBase58 extends Show[Key] {
    override def show(f: Key): String = f.asBase58
  }

  implicit val keyVectorCodec: PureCodec[Key, ByteVector] =
    PureCodec.liftEitherB(
      k ⇒ Right(k.value),
      vec ⇒
        if (vec.size == Length) Right(Key(vec))
        else Left(CodecError(s"Key length in bytes must be $Length, but ${vec.size} given"))
    )

  implicit val base58codec: PureCodec[Key, String] =
    keyVectorCodec andThen implicitly[PureCodec[ByteVector, String]]

  implicit val bytesCodec: PureCodec[Key, Array[Byte]] =
    keyVectorCodec andThen implicitly[PureCodec[ByteVector, Array[Byte]]]

  val fromBytes: PureCodec.Func[Array[Byte], Key] =
    bytesCodec.inverse

  /**
   * Tries to read base64 form of Kademlia key.
   */
  val fromB58: PureCodec.Func[String, Key] =
    base58codec.inverse

  /**
   * Calculates sha-1 hash of the payload, and wraps it with Key.
   * We keep using sha-1 instead of sha-2, because randomness is provided with keypair generation, not hash function.
   */
  val sha1: PureCodec.Func[Array[Byte], Key] =
    PureCodec.fromOtherFunc(CryptoHashers.Sha1)(err ⇒ CodecError("Crypto error", Some(err))) andThen fromBytes

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
  def checkPublicKey[F[_]: Monad](key: Key, publicKey: KeyPair.Public): EitherT[F, CryptoError, Unit] =
    EitherT.cond(
      sha1[Id](publicKey.value.toArray).value.toOption.exists(_ === key), // TODO: take error from sha1 crypto, when any
      (),
      CryptoError(s"Kademlia key doesn't match hash(pubKey); key=$key pubKey=$publicKey")
    )

}
