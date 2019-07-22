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

package fluence.kad.contact

import java.net.URI

import cats.Monad
import cats.data.EitherT
import cats.syntax.arrow._
import cats.syntax.compose._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.profunctor._
import cats.instances.option._
import fluence.codec.bits.BitsCodecs.Base58.base58ToVector
import fluence.codec.{CodecError, PureCodec}
import fluence.crypto.signature.SignAlgo.CheckerFn
import fluence.crypto.signature.{PubKeyAndSignature, Signature, Signer}
import fluence.crypto.{Crypto, CryptoError, KeyPair}
import fluence.kad.conf.AdvertizeConf
import fluence.kad.protocol.{Key, Node}
import scodec.bits.ByteVector
import PureCodec.{liftFuncEither ⇒ liftFE}
import Crypto.liftCodecErrorToCrypto

import scala.language.{higherKinds, implicitConversions}
import scala.util.Try

/**
 * URI representation of Node's contact, should be encoded as fluence://(b58 of pubKey):(b58 of signature)@host:port,
 * where (pubKey ++ host ++ port) are the signed bytes.
 *
 * @param host Host
 * @param port Port
 * @param signature Signature, along with the Public Key
 */
case class UriContact private (host: String, port: Short, signature: PubKeyAndSignature) {
  override def toString =
    s"${UriContact.Schema}://${signature.publicKey.value.toBase58}:${signature.signature.sign.toBase58}@$host:$port"

  // What's to be signed TODO build it only during signature checking, drop after that
  private[contact] lazy val msg: ByteVector =
    signature.publicKey.value ++ ByteVector(host.getBytes) ++ ByteVector.fromInt(port)
}

object UriContact {
  val Schema = "fluence"

  type ~~>[A, B] = PureCodec.Func[A, B]
  type <~>[A, B] = PureCodec[A, B]

  /**
   * Build a contact with the given params
   *
   * @param advertize Contact info to advertize through Kademlia network
   * @param signer Signer associated with this node's keypair
   */
  def buildContact(advertize: AdvertizeConf, signer: Signer): Crypto.Point[UriContact] = {
    import advertize.{host, port}
    val msg = signer.publicKey.value ++ ByteVector(host.getBytes) ++ ByteVector.fromInt(port)
    signer.signWithPK.pointAt(msg).rmap(UriContact(host, port, _))
  }

  /**
   * Build a node with the given params
   *
   * @param advertize Contact info to advertize through Kademlia network
   * @param signer Signer associated with this node's keypair
   */
  def buildNode(advertize: AdvertizeConf, signer: Signer): Crypto.Point[Node[UriContact]] =
    for {
      c ← buildContact(advertize, signer)
      k ← Crypto.fromOtherFunc(Key.fromPublicKey).pointAt(signer.publicKey)
    } yield Node(k, c)

  /**
   * Parse contact from string, check its signature
   *
   * @param checkerFn Signature checker function
   */
  def readAndCheckContact(checkerFn: CheckerFn): Crypto.Func[String, UriContact] =
    Crypto.fromOtherFunc(readContact) >>> checkContact(checkerFn)

  // codec for base58-encoded public key and signature
  val pkWithSignatureCodec: (String, String) <~> PubKeyAndSignature = {
    val signatureCodec: String <~> Signature =
      PureCodec[String, ByteVector] >>> PureCodec.liftB[ByteVector, Signature](Signature(_), _.sign)

    val pubKeyCodec: String <~> KeyPair.Public =
      PureCodec[String, ByteVector] >>> PureCodec.liftB[ByteVector, KeyPair.Public](KeyPair.Public, _.value)

    (pubKeyCodec split signatureCodec) >>> PureCodec.liftB[(KeyPair.Public, Signature), PubKeyAndSignature](
      pks ⇒ PubKeyAndSignature(pks._1, pks._2),
      pks ⇒ pks.publicKey -> pks.signature
    )
  }

  /**
   * Convert contact to string
   */
  val writeContact: UriContact ~~> String = {
    val writePks: PureCodec.Func[UriContact, String] =
      pkWithSignatureCodec.inverse.rmap(pks ⇒ s"${pks._1}:${pks._2}").lmap[UriContact](_.signature)

    PureCodec.liftFuncPoint(
      (c: UriContact) => writePks.pointAt(c).map(signature => s"${UriContact.Schema}://$signature@${c.host}:${c.port}")
    )
  }

  /**
   * Convert Node to string
   */
  implicit val writeNode: Node[UriContact] ~~> String =
    writeContact.lmap(_.contact)

  /**
   * Read Node from string, checking the signature on the way
   *
   * @param checkerFn Signature checker function
   */
  def readNode(checkerFn: CheckerFn): Crypto.Func[String, Node[UriContact]] =
    readAndCheckContact(checkerFn).rmap(c ⇒ (c, c)) >>> (
      Crypto.fromOtherFunc(Key.fromPublicKey).lmap[UriContact](_.signature.publicKey) split Crypto
        .identityFunc[UriContact]
    ).rmap {
      case (k, uc) ⇒ Node(k, uc)
    }

  // to remove PureCodec.liftFuncEither boilerplate whereas possible
  private implicit def liftEitherF[A, B](fn: A ⇒ Either[CodecError, B]): A ~~> B =
    PureCodec.liftFuncEither(fn)

  /**
   * Read the contact, performing all the formal validations on the way. Note that signature is not checked
   */
  private val readContact: String ~~> UriContact = {
    val readUri: String ~~> URI =
      liftFE[String, URI](
        s ⇒ Try(URI.create(s)).toEither.leftMap(pf ⇒ CodecError("Cannot parse string as Uri", Some(pf)))
      )

    val readHost: URI ~~> String = (uri: URI) ⇒ Either.fromOption(Option(uri.getHost), CodecError("Host not provided"))

    val readPort: URI ~~> Short = (uri: URI) ⇒
      Either
        .fromOption(Option(uri.getPort).filter(_ > 0), CodecError("Port not provided"))
        .flatMap(p ⇒ Try(p.toShort).toEither.left.map(t ⇒ CodecError(s"Port is not convertible to Short: $p", Some(t))))

    val checkScheme: URI ~~> Unit =
      (uri: URI) ⇒
        Either.fromOption(
          Option(uri.getScheme).filter(_.equalsIgnoreCase(UriContact.Schema)).void,
          CodecError(s"Uri must start with ${UriContact.Schema}://")
      )

    // PubKey and Signature are encoded as base58 in userInfo part of URI
    val readPks: URI ~~> PubKeyAndSignature = liftFE[URI, (String, String)](
      uri ⇒
        Either.fromOption(Option(uri.getUserInfo), CodecError("User info must be provided")).map(_.split(':')).flatMap {
          case Array(a, b) ⇒ Right((a, b))
          case _ ⇒ Left(CodecError("User info must be in pk:sign form"))
      }
    ) >>> pkWithSignatureCodec.direct

    // Finally, compose parsers and build the UriContact product
    readUri >>> (readHost &&& readPort &&& readPks &&& checkScheme).rmap {
      case (((host, port), pks), _) ⇒ UriContact(host, port, pks)
    }
  }

  /**
   * Check the contact's signature
   *
   * @param checkerFn Signature checker function
   */
  private def checkContact(checkerFn: CheckerFn): Crypto.Func[UriContact, UriContact] =
    new Crypto.Func[UriContact, UriContact] {
      override def apply[F[_]: Monad](input: UriContact): EitherT[F, CryptoError, UriContact] =
        checkerFn(input.signature.publicKey).check[F](input.signature.signature, input.msg).as(input)
    }

}
