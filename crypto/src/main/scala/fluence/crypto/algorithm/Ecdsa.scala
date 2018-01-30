package fluence.crypto.algorithm

import java.security.spec.{ PKCS8EncodedKeySpec, X509EncodedKeySpec }
import java.security._

import fluence.crypto.keypair.KeyPair
import org.bouncycastle.jce.ECNamedCurveTable
import scodec.bits.ByteVector

object Ecdsa {
  val ecdsa_secp256k1_sha256 = new Ecdsa("secp256k1", "SHA256withECDSA")
}

/**
 *
 * @param curveType http://www.bouncycastle.org/wiki/display/JA1/Supported+Curves+%28ECDSA+and+ECGOST%29
 * @param scheme https://bouncycastle.org/specifications.html
 */
//todo handle errors in all methods
class Ecdsa(curveType: String, scheme: String) extends SignatureFunctions {

  override def generateKeyPair(random: SecureRandom): KeyPair = {
    val ecSpec = ECNamedCurveTable.getParameterSpec(curveType)

    val g = KeyPairGenerator.getInstance("ECDSA", "BC")

    g.initialize(ecSpec, random)

    val keyPair = g.generateKeyPair()

    //todo write transformer for keypairs
    KeyPair(KeyPair.Public(ByteVector(keyPair.getPublic.getEncoded)), KeyPair.Secret(ByteVector(keyPair.getPrivate.getEncoded)))
  }

  override def generateKeyPair(): KeyPair = {
    generateKeyPair(new SecureRandom())
  }

  override def sign(keyPair: KeyPair, message: ByteVector): fluence.crypto.signature.Signature = {
    val ecdsaSign = Signature.getInstance(scheme, "BC")

    val spec = new PKCS8EncodedKeySpec(keyPair.secretKey.value.toArray)
    val factory = KeyFactory.getInstance("ECDSA")

    ecdsaSign.initSign(factory.generatePrivate(spec))
    ecdsaSign.update(message.toArray)

    fluence.crypto.signature.Signature(keyPair.publicKey, ByteVector(ecdsaSign.sign()))
  }

  override def verify(signature: fluence.crypto.signature.Signature, message: ByteVector): Boolean = {
    val ecdsaVerify = Signature.getInstance(scheme, "BC")

    val spec = new X509EncodedKeySpec(signature.publicKey.value.toArray)
    val factory = KeyFactory.getInstance("ECDSA")

    ecdsaVerify.initVerify(factory.generatePublic(spec))
    ecdsaVerify.update(message.toArray)

    ecdsaVerify.verify(signature.sign.toArray)
  }
}
