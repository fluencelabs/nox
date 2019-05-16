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
import cats.syntax.strong._
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

import scala.language.higherKinds

case class UriContact(host: String, port: Int, signature: PubKeyAndSignature) {
  override def toString = s"${signature.publicKey.value.toBase58}:${signature.signature.sign.toBase58}@$host:$port"

  lazy val msg: ByteVector = signature.publicKey.value ++ ByteVector(host.getBytes) ++ ByteVector.fromInt(port)

  def key = Key.fromPublicKey(signature.publicKey)
}

object UriContact {

  def buildContact(host: String, port: Int, signer: Signer): Crypto.Point[UriContact] = {
    val msg = signer.publicKey.value ++ ByteVector(host.getBytes) ++ ByteVector.fromInt(port)
    signer.signWithPK.pointAt(msg).rmap(UriContact(host, port, _))
  }

  implicit val signatureCodec: PureCodec[String, Signature] = PureCodec[String, ByteVector] >>> PureCodec
    .liftB[ByteVector, Signature](Signature(_), _.sign)
  implicit val pubKeyCodec: PureCodec[String, KeyPair.Public] = PureCodec[String, ByteVector] >>> PureCodec
    .liftB[ByteVector, KeyPair.Public](KeyPair.Public, _.value)

  implicit val pkWithSignatureCodec: PureCodec[(String, String), PubKeyAndSignature] =
    (pubKeyCodec split signatureCodec) >>> PureCodec.liftB[(KeyPair.Public, Signature), PubKeyAndSignature](
      pks ⇒ PubKeyAndSignature(pks._1, pks._2),
      pks ⇒ pks.publicKey -> pks.signature
    )

  val readUri: PureCodec.Func[String, Uri] =
    liftFE[String, Uri](Uri.fromString(_).leftMap(pf ⇒ CodecError("Cannot parse string as Uri", Some(pf))))

  private val readContact = {
    val readHost = liftFE[Uri, String](uri ⇒ Either.fromOption(uri.host, CodecError("Host not provided")).map(_.value))
    val readPort = liftFE[Uri, Int](uri ⇒ Either.fromOption(uri.port, CodecError("Port not provided")))
    val checkScheme = liftFE[Uri, Unit](
      uri ⇒
        Either.fromOption(
          uri.scheme.filter(_.value.equalsIgnoreCase("fluence")).void,
          CodecError("Uri must start with fluence://")
      )
    )

    val readUserInfo = liftFE[Uri, (String, String)](
      uri ⇒
        Either.fromOption(uri.userInfo, CodecError("User info must be provided")).map(_.split(':')).flatMap {
          case Array(a, b) ⇒ Right((a, b))
          case _ ⇒ Left(CodecError("User info must be in pk:sign form"))
      }
    )

    val readPks = readUserInfo >>> pkWithSignatureCodec.direct

    (readHost &&& readPort &&& readPks &&& checkScheme).rmap {
      case (((host, port), pks), _) ⇒ UriContact(host, port, pks)
    }
  }

  private def checkContact(checkerFn: CheckerFn): Crypto.Func[UriContact, Unit] =
    new Crypto.Func[UriContact, Unit] {
      override def apply[F[_]: Monad](input: UriContact): EitherT[F, CryptoError, Unit] =
        checkerFn(input.signature.publicKey).check[F](input.signature.signature, input.msg)
    }

  import Crypto.liftCodecErrorToCrypto

  def readCheckedContact(checkerFn: CheckerFn): Crypto.Func[String, UriContact] =
    Crypto.fromOtherFunc(readUri >>> readContact.rmap(i ⇒ (i, i))) >>> checkContact(checkerFn)
      .second[UriContact]
      .rmap(_._1)

  val writeContact: PureCodec.Func[UriContact, String] = {
    val writePks: PureCodec.Func[UriContact, String] =
      pkWithSignatureCodec.inverse.rmap(pks ⇒ s"${pks._1}:${pks._2}").lmap[UriContact](_.signature)

    PureCodec.liftFunc((c: UriContact) ⇒ (c, c)) >>> (writePks split PureCodec.liftFunc(identity[UriContact])).rmap {
      case (ui, uc) ⇒ s"fluence://$ui@${uc.host}:${uc.port}"
    }
  }

  val writeNode: PureCodec.Func[Node[UriContact], String] = writeContact.lmap(_.contact)

  def readNode(checkerFn: CheckerFn): Crypto.Func[String, Node[UriContact]] =
    readCheckedContact(checkerFn).rmap(c ⇒ (c, c)) >>> (
      Crypto.fromOtherFunc(Key.fromPublicKey).lmap[UriContact](_.signature.publicKey) split Crypto
        .identityFunc[UriContact]
    ).rmap {
      case (k, uc) ⇒ Node(k, uc)
    }

}
