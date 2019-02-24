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

package fluence.swarm.meta

import cats.Monad
import cats.data.EitherT
import fluence.crypto.Crypto.Hasher
import fluence.swarm.helpers.ByteVectorJsonCodec
import fluence.swarm.{MutableResourceIdentifier, SwarmConstants, SwarmError}
import io.circe.Encoder
import scodec.bits.ByteVector
import scodec.codecs.{bytes, constant, longL, _}
import shapeless.HList

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

/**
 * MetaHash required to identify and ascertain ownership of the uploadable resource.
 * We compute it as `metaHash = H(00|size|startTime|frequency|nameLength|name)`.
 * Where H() is SHA3.
 * @see [[Signature]]
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
   * @param id parameters that describe the mutable resource and required for searching updates of the mutable resource
   * @return generated hash from the input
   */
  def apply[F[_]](id: MutableResourceIdentifier)(
    implicit F: Monad[F],
    hasher: Hasher[ByteVector, ByteVector]
  ): EitherT[F, SwarmError, MetaHash] =
    for {
      nameLength <- EitherT.cond(
        id.name.forall(_.length < 255),
        id.name.map(_.length).getOrElse(0).toByte,
        SwarmError(
          "Cannot generate meta hash. The name is too big. Must be less than 255 symbols.\n" +
            s"Input: $id"
        )
      )
      binaryLength = MinimumMetadataLength + nameLength
      nameBytes = ByteVector(id.name.map(_.getBytes).getOrElse(Array.emptyByteArray))
      hashSize = (binaryLength - ChunkPrefixLength).toShort
      bytes <- codec
        .encode(
          HList(
            hashSize,
            id.startTime.toSeconds,
            id.frequency.toSeconds,
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
          id.toString +
          s", Hash: $hash"
      )
    } yield MetaHash(hash)
}
