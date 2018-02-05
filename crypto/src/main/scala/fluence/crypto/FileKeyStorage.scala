package fluence.crypto

import java.io.File
import java.nio.file.Files
import java.security
import java.security.KeyFactory
import java.security.spec.PKCS8EncodedKeySpec

import fluence.crypto.algorithm.{ Ecdsa, JavaAlgorithm }
import fluence.crypto.keypair.KeyPair
import org.bouncycastle.jce.spec.{ ECPrivateKeySpec, ECPublicKeySpec }

import scala.util.Try

class FileKeyStorage(file: File) extends JavaAlgorithm {
  def readKeyPair: Either[Throwable, KeyPair] = {
    import JavaAlgorithm._

    Try[KeyPair] {
      val keyBytes = Files.readAllBytes(file.toPath)
      val keySpec = new PKCS8EncodedKeySpec(keyBytes)

      val keyFactory = KeyFactory.getInstance(Ecdsa.ECDSA)
      val privateKey = keyFactory.generatePrivate(keySpec)

      val privSpec = keyFactory.getKeySpec(privateKey, classOf[ECPrivateKeySpec])
      val params = privSpec.getParams

      val q = params.getG.multiply(privSpec.getD)

      val pubSpec = new ECPublicKeySpec(q, params)
      val pubKey = keyFactory.generatePublic(pubSpec)

      new security.KeyPair(pubKey, privateKey)
    }.toEither
  }

  def storeSecretKey(key: KeyPair.Secret): Either[Throwable, Unit] = {
    import java.io.FileOutputStream

    Try {
      if (!file.exists()) file.createNewFile() else throw new RuntimeException(file.getAbsolutePath + " already exists")
      val fos = new FileOutputStream(file)

      fos.write(key.value.toArray)
      fos.close()
    }.toEither
  }
}
