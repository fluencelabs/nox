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

import java.net.InetAddress

import cats.data.{ EitherT, Ior }
import cats.{ Monad, Show }
import fluence.crypto.algorithm.CryptoErr
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{ SignatureChecker, Signer }
import io.circe._
import io.circe.syntax._
import scodec.bits.{ Bases, ByteVector }

import scala.language.higherKinds
import scala.util.Try

/**
 * Node contact
 *
 * @param ip IP address
 * @param port Port
 * @param publicKey Public key of the node
 * @param protocolVersion Protocol version the node is known to follow
 * @param gitHash Git hash of current build running on node
 * @param stringifier Either signer to build JWT representation, or serialized JWT, or both
 */
case class Contact(
    ip: InetAddress,
    port: Int, // httpPort, websocketPort and other transports //

    publicKey: KeyPair.Public,
    protocolVersion: Long,
    gitHash: String,

    private val stringifier: Ior[Signer, String] // TODO: make string from signer in constructor, drop signer
) {

  def b64seed[F[_] : Monad]: EitherT[F, CryptoErr, String] =
    stringifier.fold(
      s ⇒ {
        import Contact.JwtImplicits._
        Jwt.write[F].apply(jwtHeader, jwtData, s)
      },
      EitherT.pure(_),
      (_, str) ⇒ EitherT.pure(str)
    )

  private[protocol] lazy val jwtHeader =
    Contact.JwtHeader(publicKey, protocolVersion)

  private[protocol] lazy val jwtData =
    Contact.JwtData(ip, port, gitHash)

  lazy val isLocal: Boolean =
    ip.isLoopbackAddress ||
      ip.isAnyLocalAddress ||
      ip.isLinkLocalAddress ||
      ip.isSiteLocalAddress ||
      ip.isMCSiteLocal ||
      ip.isMCGlobal ||
      ip.isMCLinkLocal ||
      ip.isMCNodeLocal ||
      ip.isMCOrgLocal
}

object Contact {

  case class JwtHeader(publicKey: KeyPair.Public, protocolVersion: Long)

  case class JwtData(ip: InetAddress, port: Int, gitHash: String)

  object JwtImplicits {
    implicit val encodeHeader: Encoder[JwtHeader] = header ⇒ Json.obj(
      "pk" -> Json.fromString(header.publicKey.value.toBase64(Bases.Alphabets.Base64Url)),
      "pv" -> Json.fromLong(header.protocolVersion))

    implicit val decodeHeader: Decoder[JwtHeader] = c ⇒
      for {
        pk ← c.downField("pk").as[String]
        pv ← c.downField("pv").as[Long]
        pubKey ← ByteVector.fromBase64(pk, Bases.Alphabets.Base64Url).fold[Either[DecodingFailure, KeyPair.Public]](
          Left(DecodingFailure("Cannot parse public key", Nil))
        )(bc ⇒ Right(KeyPair.Public(bc)))
      } yield JwtHeader(pubKey, pv)

    implicit val encodeData: Encoder[JwtData] = data ⇒ Json.obj(
      "ip" -> Json.fromString(data.ip.getHostAddress),
      "p" -> Json.fromInt(data.port),
      "gh" -> Json.fromString(data.gitHash)
    )

    implicit val decodeData: Decoder[JwtData] = c ⇒
      for {
        ip ← c.downField("ip").as[String]
        p ← c.downField("p").as[Int]
        gh ← c.downField("gh").as[String]
        ipAddr ← Try(InetAddress.getByName(ip)).fold(
          t ⇒ Left(DecodingFailure.fromThrowable(t, Nil)),
          i ⇒ Right(i)
        )
      } yield JwtData(ip = ipAddr, port = p, gitHash = gh)
  }

  import JwtImplicits._

  implicit val show: Show[Contact] =
    (c: Contact) ⇒ s"${c.jwtHeader.asJson.noSpaces}.${c.jwtData.asJson.noSpaces}"

  def readB64seed[F[_] : Monad](str: String, checker: SignatureChecker): EitherT[F, Throwable, Contact] =
    Jwt.read[F, JwtHeader, JwtData](str, (h, b) ⇒ Right(h.publicKey), checker).map {
      case (header, data) ⇒
        Contact(
          ip = data.ip,
          port = data.port,
          publicKey = header.publicKey,
          protocolVersion = header.protocolVersion,
          gitHash = data.gitHash,
          stringifier = Ior.right(str)
        )
    }
}
