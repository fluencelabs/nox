package fluence.crypto.algorithm
import java.math.BigInteger
import java.security.SecureRandom

import cats.MonadError
import fluence.crypto.facade.{ Signature ⇒ JSSignature }
import fluence.crypto.facade.EC
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import scodec.bits.{ Bases, ByteVector }

import scalajs.js
import js.JSConverters._
import scala.language.higherKinds
import scala.scalajs.js
import scala.scalajs.js.typedarray.{ Int8Array, Uint8Array }

class EcdsaJS(ec: EC) extends Algorithm with SignatureFunctions with KeyGenerator {

  override def generateKeyPair[F[_]](seed: Array[Byte])(implicit F: MonadError[F, Throwable]): F[KeyPair] = {
    val seedJs = js.Dynamic.literal(entropy = seed.toJSArray)
    val key = ec.genKeyPair(Some(seedJs))
    val publ = key.getPublic(true, "hex")
    val priv = key.getPrivate("hex")
    println("secret === " + key.priv)
    println("secret2 === " + ec.keyFromPrivate(new String(priv), "hex").priv)
    F.pure(KeyPair.fromByteVectors(ByteVector.fromHex(publ).get, ByteVector.fromHex(priv).get))
  }

  override def generateKeyPair[F[_]]()(implicit F: MonadError[F, Throwable]): F[KeyPair] = generateKeyPair(Array[Byte](1, 2, 3, 4, 5))

  override def sign[F[_]](keyPair: KeyPair, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[Signature] = {
    val priv = ec.keyFromPrivate(keyPair.secretKey.value.toHex, "hex")
    println("priv === " + priv.priv)
    val sign = priv.sign(message.toArray.toJSArray).toDER("hex")
    F.pure(Signature(keyPair.publicKey, ByteVector.fromHex(sign).get))
  }

  override def verify[F[_]](signature: Signature, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[Boolean] = {
    val publ = signature.publicKey.value.toHex
    println("before")
    val pub = ec.keyFromPublic(publ, "hex")
    println("PUB === " + pub.pub)
    println("after")
    val res = pub.verify(message.toArray.toJSArray, signature.sign.toHex)
    F.pure(res)
  }

}
