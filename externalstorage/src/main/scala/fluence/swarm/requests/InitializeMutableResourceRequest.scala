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
import io.circe.generic.semiauto._
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Request for initialization a mutable resource (upload meta information and first file).
 * @param name optional resource name. You can use any name.
 * @param frequency expected time interval between updates, in seconds
 * @param startTime time the resource is valid from, in Unix time (seconds). Set to the current epoch
 *                  You can also put a startTime in the past or in the future.
 *                  Setting it in the future will prevent nodes from finding content until the clock hits startTime.
 *                  Setting it in the past allows you to create a history for the resource retroactively.
 * @param rootAddr H(ownerAddr, metaHash), where H is SHA-3 algorithm
 * @param data content the Mutable Resource will be initialized with
 * @param multiHash is a flag indicating whether the data field should be interpreted as raw data or a multihash
 * @param period Indicates for what period we are signing. Always 1 when initializing.
 * @param version Indicates what resource version of the period we are signing. Always 1 when initializing.
 * @param signature signature used to prove the owner's identity
 * @param metaHash H(size|startTime|frequency|nameLength|name), where H is SHA-3 algorithm
 * @param ownerAddr Swarm address (Ethereum wallet address)
 */
case class InitializeMutableResourceRequest private (
  name: Option[String],
  frequency: Long,
  startTime: Long,
  rootAddr: RootAddr,
  data: ByteVector,
  multiHash: Boolean,
  period: Int,
  version: Int,
  signature: Signature,
  metaHash: MetaHash,
  ownerAddr: ByteVector
)

object InitializeMutableResourceRequest {

  import MetaHash._
  import RootAddr._
  import Signature._
  import ByteVectorCodec._

  implicit val initializeRequestEncoder: Encoder[InitializeMutableResourceRequest] = deriveEncoder

  def apply[F[_]: Monad](
    name: Option[String],
    frequency: Long,
    startTime: Long,
    ownerAddr: ByteVector,
    data: ByteVector,
    multiHash: Boolean,
    signer: Signer[ByteVector, ByteVector]
  )(implicit hasher: Hasher[ByteVector, ByteVector]): EitherT[F, SwarmError, InitializeMutableResourceRequest] = {
    for {
      metaData <- Metadata.generateMetadata(name, startTime, frequency, 1, 1, multiHash, data, ownerAddr, signer)
      Metadata(metaHash, rootAddr, signature) = metaData
    } yield
      InitializeMutableResourceRequest(
        name,
        frequency,
        startTime,
        rootAddr,
        data,
        multiHash,
        1,
        1,
        signature,
        metaHash,
        ownerAddr
      )
  }
}
