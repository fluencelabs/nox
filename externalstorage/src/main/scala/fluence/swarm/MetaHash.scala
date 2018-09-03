/*
 * Copyright (C) 2018  Fluence Labs Limited
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

package fluence.swarm
import cats.Monad
import cats.data.EitherT
import fluence.crypto.Crypto.Hasher
import io.circe.Encoder
import scodec.bits.ByteVector
import scodec.codecs.{bytes, constant, longL}
import shapeless.HList
import scodec.codecs._

import scala.language.higherKinds

/**
 * MetaHash helps to identify and ascertain ownership of this resource.
 * We compute it as `metaHash = H(00|size|startTime|frequency|nameLength|name)`.
 * Where H() is SHA3.
 * @see [[fluence.swarm.Signature]]
 * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#mutable-resource-updates
 */
case class MetaHash private (hash: ByteVector)

object MetaHash extends slogging.LazyLogging {
  import SwarmConstants._
  import fluence.swarm.helpers.AttemptOps._

  private val codec = constant(ByteVector(0, 0)) :~>: short16L :: longL(64) :: longL(64) :: byte :: bytes

  implicit val metaHashEncoder: Encoder[MetaHash] = ByteVectorCodec.encodeByteVector.contramap(_.hash)

  def apply[F[_]](startTime: Long, frequency: Long, name: Option[String])(
    implicit F: Monad[F],
    hasher: Hasher[ByteVector, ByteVector]
  ): EitherT[F, SwarmError, MetaHash] =
    for {
      nameLength <- EitherT.cond(
        name.forall(_.length < 255),
        name.map(_.length).getOrElse(0).toByte,
        SwarmError("Name too big. Must be < 255")
      )
      binaryLength = minimumMetadataLength + name.map(_.length).getOrElse(0)
      nameBytes = ByteVector(name.map(_.getBytes).getOrElse(Array.emptyByteArray))
      hashSize = (binaryLength - chunkPrefixLength).toShort
      bytes <- codec
        .encode(
          HList(
            hashSize,
            startTime,
            frequency,
            nameLength,
            nameBytes
          )
        )
        .map(_.toByteVector)
        .toEitherT(er => SwarmError(s"Error on encoding metadata. ${er.messageWithContext}"))
      hash <- hasher(bytes).leftMap(er => SwarmError("Error on calculating hash for metadata.", Some(er)))
      _ = logger.debug(
        s"Generate metadata hash of name: ${name.getOrElse("<null>")}, startTime: $startTime, frequency: $frequency. " +
          s"Hash: $hash"
      )
    } yield MetaHash(hash)
}
