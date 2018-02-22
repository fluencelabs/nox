package fluence.crypto.algorithm

import java.nio.ByteBuffer
import java.security.SecureRandom

import cats.Applicative
import cats.syntax.applicative._
import fluence.crypto.cipher.ByteCrypt
import org.bouncycastle.crypto.CipherParameters
import org.bouncycastle.crypto.engines.AESEngine
import org.bouncycastle.crypto.modes.CBCBlockCipher
import org.bouncycastle.crypto.paddings.PKCS7Padding
import org.bouncycastle.crypto.paddings.PaddedBufferedBlockCipher
import org.bouncycastle.crypto.params.KeyParameter
import org.bouncycastle.crypto.params.ParametersWithIV
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.PBEKeySpec
import javax.crypto.spec.SecretKeySpec
import scodec.bits.ByteVector

import scala.language.higherKinds

object AesCrypt extends slogging.LazyLogging {

  // get raw key from password and salt// get raw key from password and salt

  val rnd = new SecureRandom()
  val fluenceSalt: ByteVector = ByteVector("fluence".getBytes())
  private def generateIV: Array[Byte] = rnd.generateSeed(16)

  def initSecretKey(password: Array[Char], salt: Array[Byte]): Array[Byte] = {
    val pbeKeySpec = new PBEKeySpec(password, salt, 50, 256)
    val keyFactory: SecretKeyFactory = SecretKeyFactory.getInstance("PBEWithSHA256And256BitAES-CBC-BC")
    val secretKey = new SecretKeySpec(keyFactory.generateSecret(pbeKeySpec).getEncoded, "AES")
    secretKey.getEncoded
  }

  def serializer[F[_] : Applicative](password: Array[Char], withIV: Boolean, salt: ByteVector = fluenceSalt): String ⇒ F[Array[Byte]] = { str ⇒
    try {
      val encData = str.getBytes()

      val key = initSecretKey(password, salt.toArray)

      val (extData, params) = if (withIV) {
        val ivData = generateIV

        // setup cipher parameters with key and IV
        val keyParam = new KeyParameter(key)
        (ivData, new ParametersWithIV(keyParam, ivData))
      } else {
        (Array.empty[Byte], new KeyParameter(key))
      }

      val cipher = setupAes(params, encrypt = true)

      // create a temporary buffer to decode into (it'll include padding)
      val buf = ByteBuffer.allocate(cipher.getOutputSize(encData.length)).array()
      val len = cipher.processBytes(encData, 0, encData.length, buf, 0)
      val len2 = len + cipher.doFinal(buf, len)

      // remove padding
      val out = ByteBuffer.allocate(len2).array()
      System.arraycopy(buf, 0, out, 0, len2)

      val serData = extData ++ buf

      serData.pure[F]
    } catch {
      case e: Throwable ⇒
        e.printStackTrace()
        throw e
    }
  }

  def setupAes(params: CipherParameters, encrypt: Boolean): PaddedBufferedBlockCipher = {
    // setup AES cipher in CBC mode with PKCS7 padding
    val padding = new PKCS7Padding
    val cipher = new PaddedBufferedBlockCipher(new CBCBlockCipher(new AESEngine), padding)
    cipher.reset()
    cipher.init(encrypt, params)

    cipher
  }

  def deserializer[F[_] : Applicative](password: Array[Char], withIV: Boolean, salt: ByteVector): Array[Byte] ⇒ F[String] = { data ⇒
    try {
      val (encData, params) = if (withIV) {
        val ivData = data.slice(0, 16)
        val encData = data.slice(16, data.length)

        val key = initSecretKey(password, salt.toArray)

        // setup cipher parameters with key and IV
        val keyParam = new KeyParameter(key)
        val params = new ParametersWithIV(keyParam, ivData)

        (encData, params)
      } else {
        val encData = data

        val key = initSecretKey(password, salt.toArray)

        // setup cipher parameters with key
        val params = new KeyParameter(key)

        (encData, params)
      }

      val cipher = setupAes(params, encrypt = false)

      // create a temporary buffer to decode into (it'll include padding)
      val buf = ByteBuffer.allocate(cipher.getOutputSize(encData.length)).array()
      val len = cipher.processBytes(encData, 0, encData.length, buf, 0)
      val len2 = len + cipher.doFinal(buf, len)

      // remove padding
      val out = ByteBuffer.allocate(len2).array()
      System.arraycopy(buf, 0, out, 0, len2)

      val str = new String(out)
      println("STR OUT === " + str)
      str.pure[F]
    } catch {
      case e: Throwable ⇒
        e.printStackTrace()
        throw e
    }
  }

  def forString[F[_] : Applicative](password: Array[Char], withIV: Boolean, salt: ByteVector = fluenceSalt): ByteCrypt[F, String] = apply[F, String](
    serializer = serializer(password, withIV, salt),
    deserializer = deserializer(password, withIV, salt)
  )

  def apply[F[_] : Applicative, T](serializer: T ⇒ F[Array[Byte]], deserializer: Array[Byte] ⇒ F[T]): ByteCrypt[F, T] =
    new ByteCrypt(serializer, deserializer)

}
