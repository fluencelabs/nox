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
import fluence.swarm.crypto.Secp256k1Signer.Signer
import fluence.swarm._
import scodec.bits.ByteVector

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

/**
 * Aggregator for all metadata for operations with Mutable Resource Updates.
 * Simplifies the use in code.
 * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#mutable-resource-updates
 *
 * @param metaHash metaHash required to identify and ascertain ownership of the uploadable resource.
 *                 metaHash = H(00|size|startTime|frequency|nameLength|name)
 *                 @see [[MetaHash]]
 * @param rootAddr rootAddr required to identify and ascertain ownership of the uploadable resource.
 *                 rootAddr = H(ownerAddr, metaHash)
 *                 @see [[RootAddr]]
 * @param signature is a signature of all inputs, metaHash and rootAddr
 *                  @see [[Signature]]
 */
case class Metadata private (metaHash: MetaHash, rootAddr: RootAddr, signature: Signature)

object Metadata {

  /**
   * Generate metadata from input. Metadata required to identify and ascertain ownership of the uploadable resource.
   *
   * @param id parameters that describe the mutable resource and required for searching updates of the mutable resource
   * @param period indicates for what period we are signing
   * @param version indicates what resource version of the period we are signing
   * @param multiHash is a flag indicating whether the data field should be interpreted as raw data or a multihash
   * @param data content the Mutable Resource will be initialized with
   * @return aggregated metadata
   */
  def generateMetadata[F[_]: Monad](
    id: MutableResourceIdentifier,
    period: Int,
    version: Int,
    multiHash: Boolean,
    data: ByteVector,
    signer: Signer[ByteVector, ByteVector]
  )(implicit hasher: Hasher[ByteVector, ByteVector]): EitherT[F, SwarmError, Metadata] =
    for {
      metaHash <- MetaHash(id)
      rootAddr <- RootAddr(metaHash, id.ownerAddr)
      signature <- Signature(period, version, rootAddr, metaHash, multiHash, data, signer)
    } yield Metadata(metaHash, rootAddr, signature)

}
