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

import scala.language.higherKinds

/**
 * RootAddr helps to identify and ascertain ownership of this resource.
 * We compute it as `rootAddr = H(ownerAddr, metaHash)`.
 * Where H() is SHA3.
 * @see [[fluence.swarm.Signature]]
 * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#mutable-resource-updates
 */
case class RootAddr private (addr: ByteVector)

object RootAddr extends slogging.LazyLogging {
  implicit val rootAddrEncoder: Encoder[RootAddr] = ByteVectorCodec.encodeByteVector.contramap(_.addr)

  def apply[F[_]: Monad](metaHash: MetaHash, ownerAddr: ByteVector)(
    implicit hasher: Hasher[ByteVector, ByteVector]
  ): EitherT[F, SwarmError, RootAddr] =
    hasher(metaHash.hash ++ ownerAddr)
      .map(RootAddr.apply)
      .leftMap(er => SwarmError("Error on generating root address.", Some(er)))
      .map { rootAddr =>
        logger.debug(
          s"Generate rootAddr hash of metaHash: ${metaHash.hash} and ownerAddr: $ownerAddr. Hash: ${rootAddr.addr}"
        )
        rootAddr
      }
}
