/*
 * Copyright (C) 2017  Fluence Labs Limited
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

package fluence.kad.protocol

import cats.data.{EitherT, Reader}
import cats.{Monad, Show}
import fluence.codec
import fluence.codec.{CodecError, PureCodec}
import fluence.crypto.{CryptoError, KeyPair}
import fluence.crypto.signature.SignAlgo.CheckerFn
import fluence.crypto.signature.Signer
import io.circe._
import scodec.bits.{Bases, ByteVector}

import scala.language.higherKinds

/**
 * Node contact
 *
 * @param addr IP address
 * @param grpcPort Port for GRPC server
 * @param publicKey Public key of the node
 * @param protocolVersion Protocol version the node is known to follow
 * @param gitHash Git hash of current build running on node
 * @param b64seed Serialized JWT
 */
case class Contact(
  addr: String,
  grpcPort: Int, // httpPort, websocketPort and other transports //
  websocketPort: Option[Int],
  publicKey: KeyPair.Public,
  protocolVersion: Long,
  gitHash: String,
  b64seed: String
)

object Contact {
  implicit val publicKeyR: Reader[Contact, KeyPair.Public] = Reader(_.publicKey)
  implicit val addrR: Reader[Contact, String] = Reader(_.addr)

  implicit def contactBytesCodec(implicit checkerFn: CheckerFn): PureCodec[Contact, Array[Byte]] =
    PureCodec.Bijection(
      new codec.PureCodec.Func[Contact, Array[Byte]] {
        override def apply[F[_]: Monad](input: Contact): EitherT[F, CodecError, Array[Byte]] =
          EitherT.rightT(input.b64seed.getBytes())
      },
      new codec.PureCodec.Func[Array[Byte], Contact] {
        override def apply[F[_]: Monad](input: Array[Byte]): EitherT[F, CodecError, Contact] = {
          EitherT
            .rightT[F, Throwable](new String(input))
            .flatMap(readB64seed[F])
            .leftMap(t ⇒ CodecError("Cannot decode Contact", causedBy = Some(t)))
        }
      }
    )

  /**
   * Builder for Node's own contact: node don't have JWT seed for it, but can produce it with its Signer
   *
   * @param addr Node's ip
   * @param port Node's external port
   * @param protocolVersion Protocol version
   * @param gitHash Current build's git hash
   * @param signer Node's signer
   * @tparam F Monad
   * @return Either Contact if built, or error
   */
  def buildOwn[F[_]: Monad](
    addr: String,
    port: Int, // httpPort, websocketPort and other transports //
    websocketPort: Option[Int],
    protocolVersion: Long,
    gitHash: String,
    signer: Signer
  ): EitherT[F, CryptoError, Contact] = {
    val jwtHeader =
      Contact.JwtHeader(signer.publicKey, protocolVersion)

    val jwtData =
      Contact.JwtData(addr, port, websocketPort, gitHash)

    import Contact.JwtImplicits._

    Jwt.write[F].apply(jwtHeader, jwtData, signer).map { seed ⇒
      Contact(
        addr,
        port,
        websocketPort: Option[Int],
        signer.publicKey,
        protocolVersion,
        gitHash,
        seed
      )
    }
  }

  case class JwtHeader(publicKey: KeyPair.Public, protocolVersion: Long)

  case class JwtData(addr: String, grpcPort: Int, websocketPort: Option[Int], gitHash: String)

  object JwtImplicits {
    implicit val encodeHeader: Encoder[JwtHeader] = header ⇒
      Json.obj(
        "pk" -> Json.fromString(header.publicKey.value.toBase64(Bases.Alphabets.Base64Url)),
        "pv" -> Json.fromLong(header.protocolVersion)
    )

    implicit val decodeHeader: Decoder[JwtHeader] = c ⇒
      for {
        pk ← c.downField("pk").as[String]
        pv ← c.downField("pv").as[Long]
        pubKey ← ByteVector
          .fromBase64(pk, Bases.Alphabets.Base64Url)
          .fold[Either[DecodingFailure, KeyPair.Public]](
            Left(DecodingFailure("Cannot parse public key", Nil))
          )(bc ⇒ Right(KeyPair.Public(bc)))
      } yield JwtHeader(pubKey, pv)

    implicit val encodeData: Encoder[JwtData] = data ⇒
      Json.obj(
        "a" -> Json.fromString(data.addr),
        "gp" -> Json.fromInt(data.grpcPort),
        "gh" -> Json.fromString(data.gitHash),
        "wp" -> data.websocketPort.map(Json.fromInt).getOrElse(Json.Null)
    )

    implicit val decodeData: Decoder[JwtData] = c ⇒
      for {
        addr ← c.downField("a").as[String]
        p ← c.downField("gp").as[Int]
        gh ← c.downField("gh").as[String]
        wp ← c.downField("wp").as[Option[Int]]
      } yield JwtData(addr = addr, grpcPort = p, websocketPort = wp, gitHash = gh)
  }

  import JwtImplicits._

  implicit val show: Show[Contact] =
    (c: Contact) ⇒ s"$c"

  def readB64seed[F[_]: Monad](str: String)(implicit checkerFn: CheckerFn): EitherT[F, Throwable, Contact] =
    Jwt.read[F, JwtHeader, JwtData](str, (h, b) ⇒ Right(h.publicKey)).map {
      case (header, data) ⇒
        Contact(
          addr = data.addr,
          grpcPort = data.grpcPort,
          websocketPort = data.websocketPort,
          publicKey = header.publicKey,
          protocolVersion = header.protocolVersion,
          gitHash = data.gitHash,
          b64seed = str
        )
    }
}
