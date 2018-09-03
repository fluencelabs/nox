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
import fluence.swarm.ECDSASigner.Signer
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Aggregator for all metadata for operations with Mutable Resource Updates.
 * @see [[fluence.swarm.Signature]]
 * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#mutable-resource-updates
 *
 */
case class Metadata private (metaHash: MetaHash, rootAddr: RootAddr, signature: Signature)

object Metadata {

  def generateMetadata[F[_]: Monad](
    name: Option[String],
    startTime: Long,
    frequency: Long,
    period: Int,
    version: Int,
    multiHash: Boolean,
    data: ByteVector,
    ownerAddr: ByteVector,
    signer: Signer[ByteVector, ByteVector]
  )(implicit hasher: Hasher[ByteVector, ByteVector]): EitherT[F, SwarmError, Metadata] = {

    for {
      metaHash <- MetaHash(startTime, frequency, name)
      rootAddr <- RootAddr(metaHash, ownerAddr)
      signature <- Signature(period, version, rootAddr, metaHash, multiHash, data, signer)
    } yield Metadata(metaHash, rootAddr, signature)
  }
}
