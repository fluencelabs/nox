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

package fluence.kad.protocol

import cats.data.Reader
import cats.Show
import cats.syntax.functor._
import cats.syntax.profunctor._
import cats.syntax.strong._
import cats.syntax.compose._
import fluence.codec.{CodecError, PureCodec}
import fluence.crypto.jwt.CryptoJwt
import fluence.crypto.{Crypto, KeyPair}
import fluence.crypto.signature.SignAlgo.CheckerFn
import fluence.crypto.signature.Signer
import io.circe._
import scodec.bits.{Bases, ByteVector}

import scala.language.higherKinds

/**
 * Node contact
 *
 * @param addr            IP address
 * @param httpPort        Port for HTTP server
 * @param publicKey       Public key of the node
 * @param protocolVersion Protocol version the node is known to follow
 * @param gitHash         Git hash of current build running on node
 * @param b64seed         Serialized JWT
 */
case class Contact(
  addr: String,
  httpPort: Int,
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
      PureCodec
        .liftFunc(_.b64seed.getBytes()),
      PureCodec
        .fromOtherFunc(readB64seed)(e ⇒ CodecError("JWT token verification failed", Some(e)))
        .lmap[Array[Byte]](new String(_))
    )

  case class JwtHeader(publicKey: KeyPair.Public, protocolVersion: Long)

  case class JwtData(addr: String, httpPort: Int, gitHash: String)

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
        "hp" -> Json.fromInt(data.httpPort),
        "gh" -> Json.fromString(data.gitHash)
    )

    implicit val decodeData: Decoder[JwtData] = c ⇒
      for {
        addr ← c.downField("a").as[String]
        p ← c.downField("hp").as[Int]
        gh ← c.downField("gh").as[String]
      } yield JwtData(addr = addr, httpPort = p, gitHash = gh)
  }

  val cryptoJwt: CryptoJwt[JwtHeader, JwtData] = {
    import JwtImplicits._

    new CryptoJwt[JwtHeader, JwtData](PureCodec.liftFunc(_._1.publicKey))
  }

  implicit val show: Show[Contact] =
    (c: Contact) ⇒ s"$c"

  /**
   * Builder for Node's own contact: node don't have JWT seed for it, but can produce it with its Signer
   *
   * @param addr Node's ip
   * @param port Node's external port
   * @param protocolVersion Protocol version
   * @param gitHash Current build's git hash
   * @param signer Node's signer
   * @return Either Contact if built, or error
   */
  def buildOwn(
    addr: String,
    port: Int, // httpPort, websocketPort and other transports //
    protocolVersion: Long,
    gitHash: String,
    signer: Signer
  ): Crypto.Point[Contact] = {
    val jwtHeader =
      Contact.JwtHeader(signer.publicKey, protocolVersion)

    val jwtData =
      Contact.JwtData(addr, port, gitHash)

    cryptoJwt
      .writer(signer)
      .pointAt(jwtHeader → jwtData)
      .map { seed ⇒
        Contact(addr, port, signer.publicKey, protocolVersion, gitHash, seed)
      }
  }

  /**
   * Parse contact's JWT, using signature checker
   *
   * @param checkerFn Signature checker
   * @return Func from JWT to Contact
   */
  def readB64seed(implicit checkerFn: CheckerFn): Crypto.Func[String, Contact] =
    (Crypto.liftFunc[String, (String, String)](str ⇒ (str, str)) andThen cryptoJwt
      .reader(checkerFn)
      .first[String]).rmap {
      case ((header, data), str) ⇒
        Contact(
          addr = data.addr,
          httpPort = data.httpPort,
          publicKey = header.publicKey,
          protocolVersion = header.protocolVersion,
          gitHash = data.gitHash,
          b64seed = str
        )
    }
}
