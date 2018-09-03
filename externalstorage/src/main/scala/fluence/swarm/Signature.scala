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
import cats.{Id, Monad}
import cats.data.EitherT
import fluence.crypto.Crypto.Hasher
import fluence.swarm.ECDSASigner.Signer
import io.circe.Encoder
import scodec.bits.ByteVector
import scodec.codecs._
import shapeless.HList

import scala.language.{higherKinds, implicitConversions}

/**
 * Signature helps to identify and ascertain ownership of this resource.
 * Update chunks must carry a rootAddr reference and metaHash in order to be verified.
 * This way, a node that receives an update can check the signature, recover the public address
 * and check the ownership by computing H(ownerAddr, metaHash) and comparing it to the rootAddr
 * the resource is claiming to update without having to lookup the metadata chunk.
 * It is a sign of `digest`.
 * `digest = H(period|version|rootAddr|metaHash|multihash|data)`
 * Where H() is SHA3.
 * `period` version are encoded as little-endian long
 * `rootAddr` is encoded as a 32 byte array
 * `metaHash` is encoded as a 32 byte array
 * `multihash` is encoded as the least significant bit of a flags byte
 * `data` is the plain data byte array
 * @see https://swarm-guide.readthedocs.io/en/latest/usage.html#mutable-resource-updates
 */
case class Signature private (sign: ByteVector)

object Signature extends slogging.LazyLogging {

  import fluence.swarm.helpers.AttemptOps._
  import SwarmConstants._

  implicit val signatureEncoder: Encoder[Signature] = ByteVectorCodec.encodeByteVector.contramap(_.sign)

  private val codec = short16L :: short16L :: int32L :: int32L :: bytes :: bytes :: bool(8) :: bytes

  private def binaryLength(data: ByteVector): Int = chunkPrefixLength + updateHeaderLength + data.size.toInt

  def apply[F[_]: Monad](
    period: Int,
    version: Int,
    rootAddr: RootAddr,
    metaHash: MetaHash,
    multiHash: Boolean,
    data: ByteVector,
    signer: Signer[ByteVector, ByteVector]
  )(implicit hasher: Hasher[ByteVector, ByteVector]): EitherT[F, SwarmError, Signature] = {
    for {
      bytes <- codec
        .encode(
          HList(
            updateHeaderLength,
            data.size.toShort,
            period,
            version,
            rootAddr.addr,
            metaHash.hash,
            multiHash,
            data
          )
        )
        .map(_.toByteVector)
        .toEitherT(er => SwarmError(s"Error on encoding signature. ${er.messageWithContext}"))
      _ = logger.debug(
        s"Generate signature of period: $period, version: $version, rootAddr: ${rootAddr.addr}," +
          s"metaHash: ${metaHash.hash}, multiHash: $multiHash, data: $data"
      )
      digestHash <- hasher(bytes).leftMap(er => SwarmError("Error on hashing signature.", Some(er)))
      _ = logger.debug(s"Digest hash on generating signature: $digestHash")
      sign <- signer(digestHash).leftMap(er => SwarmError("Error on sign.", Some(er)))
      _ = logger.debug(s"Generated sign: $sign")
    } yield Signature(sign)
  }
}
