package fluence.crypto.algorithm

import java.security.SecureRandom

import cats.{ Applicative, Monad, MonadError }
import cats.data.EitherT
import cats.syntax.flatMap._
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

class AesCrypt[F[_] : Monad, T](password: Array[Char], withIV: Boolean, salt: ByteVector)(serializer: T ⇒ F[Array[Byte]], deserializer: Array[Byte] ⇒ F[T])(implicit ME: MonadError[F, Throwable])
  extends ByteCrypt[F, T](serializer, deserializer) {
  import CryptoErr._
  // get raw key from password and salt// get raw key from password and salt

  val rnd = new SecureRandom()
  private def generateIV: Array[Byte] = rnd.generateSeed(16)

  def initSecretKey(password: Array[Char], salt: Array[Byte]): EitherT[F, CryptoErr, Array[Byte]] = {
    nonFatalHandling {
      val pbeKeySpec = new PBEKeySpec(password, salt, 50, 256)
      val keyFactory: SecretKeyFactory = SecretKeyFactory.getInstance("PBEWithSHA256And256BitAES-CBC-BC")
      val secretKey = new SecretKeySpec(keyFactory.generateSecret(pbeKeySpec).getEncoded, "AES")
      secretKey.getEncoded
    }("Cannot init secret key.")
  }

  def setupAes(params: CipherParameters, encrypt: Boolean): EitherT[F, CryptoErr, PaddedBufferedBlockCipher] = {
    nonFatalHandling {
      // setup AES cipher in CBC mode with PKCS7 padding
      val padding = new PKCS7Padding
      val cipher = new PaddedBufferedBlockCipher(new CBCBlockCipher(new AESEngine), padding)
      cipher.reset()
      cipher.init(encrypt, params)

      cipher
    }("Cannot setup aes cipher.")
  }

  def processBytes(data: Array[Byte], cipher: PaddedBufferedBlockCipher): EitherT[F, CryptoErr, Array[Byte]] = {
    nonFatalHandling {
      // create a temporary buffer to decode into (it'll include padding)
      val buf = new Array[Byte](data.length)
      println("cpiher process")

      val len = cipher.processBytes(data, 0, data.length, buf, 0)

      val len2 = len + cipher.doFinal(buf, len)

      buf.slice(0, len2)
    }("Error in process bytes")
  }

  def deserializer(password: Array[Char], withIV: Boolean, salt: ByteVector): Array[Byte] ⇒ F[String] = { data ⇒
    val e = for {
      encDataParams ← if (withIV) {
        withIVGen(data, password, salt)
      } else {
        withoutIVGen(data, password, salt)
      }
      (detachedData, params) = encDataParams
      out ← processData(detachedData, None, params, encrypt = false)
      _ = println("OUT === " + out)
    } yield new String(out)

    e.value.flatMap(ME.fromEither)
  }

  def processData(data: Array[Byte], extData: Option[Array[Byte]], params: CipherParameters, encrypt: Boolean): EitherT[F, CryptoErr, Array[Byte]] = {
    for {
      cipher ← setupAes(params, encrypt = true)
      buf ← processBytes(data, cipher)
      serData = extData.map(_ ++ buf).getOrElse(buf)
    } yield serData
  }

  def serializer(password: Array[Char], withIV: Boolean, salt: ByteVector = AesCrypt.fluenceSalt): String ⇒ F[Array[Byte]] = { str ⇒

    val encData = str.getBytes()

    val e = for {
      key ← initSecretKey(password, salt.toArray)
      (extData, params) = {
        if (withIV) {
          val ivData = generateIV

          // setup cipher parameters with key and IV
          val keyParam = new KeyParameter(key)
          (Some(ivData), new ParametersWithIV(keyParam, ivData))
        } else {
          (None, new KeyParameter(key))
        }
      }
      serData ← processData(encData, extData, params, encrypt = true)
      _ = println("SER DATA === " + serData)
    } yield serData

    e.value.flatMap(ME.fromEither)
  }

  def withIVGen(data: Array[Byte], password: Array[Char], salt: ByteVector): EitherT[F, CryptoErr, (Array[Byte], CipherParameters)] = {
    val ivData = data.slice(0, 16)
    val encData = data.slice(16, data.length)
    for {
      key ← initSecretKey(password, salt.toArray)
      // setup cipher parameters with key and IV
      keyParam = new KeyParameter(key)
      params = new ParametersWithIV(keyParam, ivData)
    } yield (encData, params)
  }

  def withoutIVGen(data: Array[Byte], password: Array[Char], salt: ByteVector): EitherT[F, CryptoErr, (Array[Byte], CipherParameters)] = {
    val encData = data
    for {
      key ← initSecretKey(password, salt.toArray)
      // setup cipher parameters with key
      params = new KeyParameter(key)
    } yield (encData, params)
  }
}

object AesCrypt extends slogging.LazyLogging {

  val fluenceSalt: ByteVector = ByteVector("fluence".getBytes())

  def forString[F[_] : Applicative](password: Array[Char], withIV: Boolean, salt: ByteVector = fluenceSalt)(implicit ME: MonadError[F, Throwable]): ByteCrypt[F, String] =
    apply[F, String](password, withIV, salt)(
      serializer = _.getBytes.pure[F],
      deserializer = bytes ⇒ new String(bytes).pure[F]
    )

  def apply[F[_] : Applicative, T](password: Array[Char], withIV: Boolean, salt: ByteVector)(serializer: T ⇒ F[Array[Byte]], deserializer: Array[Byte] ⇒ F[T])(implicit ME: MonadError[F, Throwable]): ByteCrypt[F, T] =
    new AesCrypt(password, withIV, salt)(serializer, deserializer)

}
