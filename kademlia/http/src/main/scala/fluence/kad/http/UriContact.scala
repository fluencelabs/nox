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

package fluence.kad.http

import cats.Monad
import cats.data.EitherT
import fluence.codec.{CodecError, PureCodec}
import fluence.crypto.{Crypto, CryptoError, KeyPair}
import fluence.crypto.signature.{PubKeyAndSignature, Signature, Signer}
import cats.syntax.compose._
import cats.syntax.arrow._
import cats.syntax.flatMap._
import cats.syntax.profunctor._
import cats.syntax.either._
import cats.syntax.functor._
import cats.instances.option._
import fluence.codec.bits.BitsCodecs
import fluence.crypto.signature.SignAlgo.CheckerFn
import fluence.kad.protocol.{Key, Node}
import org.http4s.Uri
import scodec.bits.ByteVector
import BitsCodecs.Base58.base58ToVector
import PureCodec.{liftFuncEither ⇒ liftFE}
import Crypto.liftCodecErrorToCrypto

import scala.language.higherKinds
import scala.language.implicitConversions
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
    s"fluence://${signature.publicKey.value.toBase58}:${signature.signature.sign.toBase58}@$host:$port"

  // What's to be signed TODO build it only during signature checking, drop after that
  private[http] lazy val msg: ByteVector =
    signature.publicKey.value ++ ByteVector(host.getBytes) ++ ByteVector.fromInt(port)
}

object UriContact {
  type ~~>[A, B] = PureCodec.Func[A, B]
  type <~>[A, B] = PureCodec[A, B]

  /**
   * Build a contact with the given params
   *
   * @param host Host
   * @param port Port
   * @param signer Signer associated with this node's keypair
   */
  def buildContact(host: String, port: Short, signer: Signer): Crypto.Point[UriContact] = {
    val msg = signer.publicKey.value ++ ByteVector(host.getBytes) ++ ByteVector.fromInt(port)
    signer.signWithPK.pointAt(msg).rmap(UriContact(host, port, _))
  }

  /**
   * Build a node with the given params
   *
   * @param host Host
   * @param port Port
   * @param signer Signer associated with this node's keypair
   */
  def buildNode(host: String, port: Short, signer: Signer): Crypto.Point[Node[UriContact]] =
    for {
      c ← buildContact(host, port, signer)
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

    PureCodec.liftFunc((c: UriContact) ⇒ (c, c)) >>> (writePks split PureCodec.liftFunc(identity[UriContact])).rmap {
      case (ui, uc) ⇒ s"fluence://$ui@${uc.host}:${uc.port}"
    }
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
    val readUri: String ~~> Uri =
      liftFE[String, Uri](Uri.fromString(_).leftMap(pf ⇒ CodecError("Cannot parse string as Uri", Some(pf))))

    val readHost: Uri ~~> String = (uri: Uri) ⇒
      Either.fromOption(uri.host, CodecError("Host not provided")).map(_.value)

    val readPort: Uri ~~> Short = (uri: Uri) ⇒
      Either
        .fromOption(uri.port, CodecError("Port not provided"))
        .flatMap(p ⇒ Try(p.toShort).toEither.left.map(t ⇒ CodecError(s"Port is not convertible to Short: $p")))

    val checkScheme: Uri ~~> Unit =
      (uri: Uri) ⇒
        Either.fromOption(
          uri.scheme.filter(_.value.equalsIgnoreCase("fluence")).void,
          CodecError("Uri must start with fluence://")
      )

    // PubKey and Signature are encoded as base58 in userInfo part of URI
    val readPks: Uri ~~> PubKeyAndSignature = liftFE[Uri, (String, String)](
      uri ⇒
        Either.fromOption(uri.userInfo, CodecError("User info must be provided")).map(_.split(':')).flatMap {
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
