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

package fluence.swarm.requests

import cats.Monad
import cats.data.EitherT
import fluence.crypto.Crypto.Hasher
import fluence.swarm.ECDSASigner.Signer
import fluence.swarm._
import io.circe.Encoder
import io.circe.generic.semiauto.deriveEncoder
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Request for updating a mutable resource.
 *
 * @param rootAddr H(ownerAddr, metaHash), where H is SHA-3 algorithm
 * @param metaHash H(size|startTime|frequency|nameLength|name), where H is SHA-3 algorithm
 * @param period Indicates for what period we are signing.
 * @param version Indicates what resource version of the period we are signing.
 * @param data content the Mutable Resource will be initialized with
 * @param multiHash is a flag indicating whether the data field should be interpreted as raw data or a multihash
 * @param signature signature used to prove the owner's identity
 */
case class UpdateMutableResourceRequest private (
  rootAddr: RootAddr,
  metaHash: MetaHash,
  period: Int,
  version: Int,
  data: ByteVector,
  multiHash: Boolean,
  signature: Signature
)

object UpdateMutableResourceRequest {
  import MetaHash._
  import RootAddr._
  import Signature._
  import ByteVectorCodec._

  implicit val updateRequestEncoder: Encoder[UpdateMutableResourceRequest] = deriveEncoder

  def apply[F[_]: Monad](
    name: Option[String],
    frequency: Long,
    startTime: Long,
    ownerAddr: ByteVector,
    data: ByteVector,
    multiHash: Boolean,
    period: Int,
    version: Int,
    signer: Signer[ByteVector, ByteVector]
  )(implicit hasher: Hasher[ByteVector, ByteVector]): EitherT[F, SwarmError, UpdateMutableResourceRequest] = {
    for {
      metaData <- Metadata
        .generateMetadata(name, startTime, frequency, period, version, multiHash, data, ownerAddr, signer)
      Metadata(metaHash, rootAddr, signature) = metaData
    } yield UpdateMutableResourceRequest(rootAddr, metaHash, period, version, data, multiHash, signature)

  }
}
