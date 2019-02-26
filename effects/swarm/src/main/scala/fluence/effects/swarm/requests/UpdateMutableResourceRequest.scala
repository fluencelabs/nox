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

package fluence.effects.swarm.requests

import cats.Monad
import cats.data.EitherT
import fluence.crypto.Crypto.Hasher
import fluence.effects.swarm.crypto.Secp256k1Signer.Signer
import fluence.effects.swarm._
import fluence.effects.swarm.meta.{MetaHash, Metadata, RootAddr, Signature}
import io.circe.Encoder
import io.circe.generic.semiauto.deriveEncoder
import scodec.bits.ByteVector

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

/**
 * Request for updating a mutable resource.
 *
 * @param rootAddr H(ownerAddr, metaHash), where H is SHA-3 algorithm
 * @param metaHash H(size|startTime|frequency|nameLength|name), where H is SHA-3 algorithm
 * @param period indicates for what period we are signing
 * @param version indicates what resource version of the period we are signing
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
  import fluence.effects.swarm.helpers.ByteVectorJsonCodec._

  implicit val updateRequestEncoder: Encoder[UpdateMutableResourceRequest] = deriveEncoder

  def apply[F[_]: Monad](
    id: MutableResourceIdentifier,
    data: ByteVector,
    multiHash: Boolean,
    period: Int,
    version: Int,
    signer: Signer[ByteVector, ByteVector]
  )(implicit hasher: Hasher[ByteVector, ByteVector]): EitherT[F, SwarmError, UpdateMutableResourceRequest] = {
    for {
      metaData <- Metadata
        .generateMetadata(id, period, version, multiHash, data, signer)
      Metadata(metaHash, rootAddr, signature) = metaData
    } yield UpdateMutableResourceRequest(rootAddr, metaHash, period, version, data, multiHash, signature)

  }
}
