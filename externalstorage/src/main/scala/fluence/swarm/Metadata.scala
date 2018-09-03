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
