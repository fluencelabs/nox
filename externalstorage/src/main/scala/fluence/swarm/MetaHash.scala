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

package fluence.swarm
import cats.Monad
import cats.data.EitherT
import fluence.crypto.Crypto.Hasher
import io.circe.Encoder
import scodec.bits.ByteVector
import scodec.codecs.{bytes, constant, longL}
import shapeless.HList
import scodec.codecs._

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

/**
 * MetaHash required to identify and ascertain ownership of the uploadable resource.
 * We compute it as `metaHash = H(00|size|startTime|frequency|nameLength|name)`.
 * Where H() is SHA3.
 * @see [[fluence.swarm.Signature]]
 * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#mutable-resource-updates
 */
case class MetaHash private (hash: ByteVector)

object MetaHash extends slogging.LazyLogging {
  import SwarmConstants._
  import fluence.swarm.helpers.AttemptOps._

  // 00 | size | startTime | frequency | nameLength | name
  private val codec = constant(ByteVector(0, 0)) :~>: short16L :: longL(64) :: longL(64) :: byte :: bytes

  implicit val metaHashEncoder: Encoder[MetaHash] = ByteVectorJsonCodec.encodeByteVector.contramap(_.hash)

  /**
   *
   * @param startTime time the resource is valid from, in Unix time (seconds). Set to the current epoch
   * @param frequency expected time interval between updates, in seconds
   * @param name resource name. This is a user field. It can be any name
   * @return generated hash from the input
   */
  def apply[F[_]](startTime: FiniteDuration, frequency: FiniteDuration, name: Option[String])(
    implicit F: Monad[F],
    hasher: Hasher[ByteVector, ByteVector]
  ): EitherT[F, SwarmError, MetaHash] =
    for {
      nameLength <- EitherT.cond(
        name.forall(_.length < 255),
        name.map(_.length).getOrElse(0).toByte,
        SwarmError("Cannot generate meta hash. The name is too big. Must be less than 255 symbols.\n" +
          s"Input: $startTime, frequency: $frequency, name: $name")
      )
      binaryLength = MinimumMetadataLength + nameLength
      nameBytes = ByteVector(name.map(_.getBytes).getOrElse(Array.emptyByteArray))
      hashSize = (binaryLength - ChunkPrefixLength).toShort
      bytes <- codec
        .encode(
          HList(
            hashSize,
            startTime.toSeconds,
            frequency.toSeconds,
            nameLength,
            nameBytes
          )
        )
        .map(_.toByteVector)
        .toEitherT(er => SwarmError(s"Error on encoding metadata. ${er.messageWithContext}"))

      hash <- hasher(bytes)
        .leftMap(er => SwarmError("Error on calculating hash for metadata.", Some(er)))

      _ = logger.debug(
        s"Generate metadata hash of " +
          s"name: ${name.getOrElse("<null>")}, " +
          s"startTime: $startTime, " +
          s"frequency: $frequency. " +
          s"Hash: $hash"
      )
    } yield MetaHash(hash)
}
