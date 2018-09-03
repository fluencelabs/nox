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
      .leftMap(er => SwarmError("Error on generating rood address."))
      .map { rootAddr =>
        logger.debug(
          s"Generate rootAddr hash of metaHash: ${metaHash.hash} and ownerAddr: $ownerAddr. Hash: ${rootAddr.addr}"
        )
        rootAddr
      }
}
