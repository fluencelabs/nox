package fluence.crypto.algorithm

import java.security.Security

import fluence.crypto.keypair.KeyPair
import org.bouncycastle.jce.provider.BouncyCastleProvider
import scodec.bits.ByteVector

private[algorithm] trait JavaAlgorithm[F[_]] extends Algorithm[F] {
  JavaAlgorithm.addProvider
}

object JavaAlgorithm {
  implicit def jKeyPairToKeyPair(jKeyPair: java.security.KeyPair): KeyPair =
    KeyPair(KeyPair.Public(ByteVector(jKeyPair.getPublic.getEncoded)), KeyPair.Secret(ByteVector(jKeyPair.getPrivate.getEncoded)))

  private lazy val addProvider = Security.addProvider(new BouncyCastleProvider())
}
