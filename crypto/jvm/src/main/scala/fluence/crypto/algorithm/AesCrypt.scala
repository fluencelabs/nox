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

case class DetachedData(ivData: Array[Byte], encData: Array[Byte])
case class DataWithParams(data: Array[Byte], params: CipherParameters)

class AesCrypt[F[_] : Monad, T](password: Array[Char], withIV: Boolean, salt: ByteVector)(serializer: T ⇒ F[Array[Byte]], deserializer: Array[Byte] ⇒ F[T])(implicit ME: MonadError[F, Throwable])
  extends ByteCrypt[F, T](serializer, deserializer) {
  import CryptoErr._
  // get raw key from password and salt// get raw key from password and salt

  val rnd = new SecureRandom()
  val BITS = 256
  val ITERATION_COUNT = 50
  val IV_SIZE = 16
  private def generateIV: Array[Byte] = rnd.generateSeed(IV_SIZE)

  override def encrypt(plainText: T): F[Array[Byte]] = {
    val e = for {
      data ← EitherT.liftF(serializer(plainText))
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
      encData ← processData(DataWithParams(data, params), extData, encrypt = true)
    } yield encData

    e.value.flatMap(ME.fromEither)
  }

  override def decrypt(cipherText: Array[Byte]): F[T] = {
    val e = for {
      dataWithParams ← detachDataAndGetParams(cipherText, password, salt, withIV)
      decData ← processData(dataWithParams, None, encrypt = false)
      plain ← EitherT.liftF[F, CryptoErr, T](deserializer(decData))
    } yield plain

    e.value.flatMap(ME.fromEither)
  }

  private def initSecretKey(password: Array[Char], salt: Array[Byte]): EitherT[F, CryptoErr, Array[Byte]] = {
    nonFatalHandling {
      val pbeKeySpec = new PBEKeySpec(password, salt, ITERATION_COUNT, BITS)
      val keyFactory: SecretKeyFactory = SecretKeyFactory.getInstance("PBEWithSHA256And256BitAES-CBC-BC")
      val secretKey = new SecretKeySpec(keyFactory.generateSecret(pbeKeySpec).getEncoded, "AES")
      secretKey.getEncoded
    }("Cannot init secret key.")
  }

  private def setupAes(params: CipherParameters, encrypt: Boolean): EitherT[F, CryptoErr, PaddedBufferedBlockCipher] = {
    nonFatalHandling {
      // setup AES cipher in CBC mode with PKCS7 padding
      val padding = new PKCS7Padding
      val cipher = new PaddedBufferedBlockCipher(new CBCBlockCipher(new AESEngine), padding)
      cipher.reset()
      cipher.init(encrypt, params)

      cipher
    }("Cannot setup aes cipher.")
  }

  private def processBytes(data: Array[Byte], cipher: PaddedBufferedBlockCipher): EitherT[F, CryptoErr, Array[Byte]] = {
    nonFatalHandling {
      // create a temporary buffer to decode into (it'll include padding)
      val buf = new Array[Byte](cipher.getOutputSize(data.length))
      val outputLength = cipher.processBytes(data, 0, data.length, buf, 0)
      val lastBlockLength = cipher.doFinal(buf, outputLength)
      //remove padding
      buf.slice(0, outputLength + lastBlockLength)
    }("Error in cipher processing")
  }

  private def processData(dataWithParams: DataWithParams, extData: Option[Array[Byte]], encrypt: Boolean): EitherT[F, CryptoErr, Array[Byte]] = {
    for {
      cipher ← setupAes(dataWithParams.params, encrypt = encrypt)
      buf ← processBytes(dataWithParams.data, cipher)
      serData = extData.map(_ ++ buf).getOrElse(buf)
    } yield serData
  }

  private def detachIV(data: Array[Byte], ivSize: Int): EitherT[F, CryptoErr, DetachedData] = {
    nonFatalHandling {
      val ivData = data.slice(0, ivSize)
      val encData = data.slice(ivSize, data.length)
      DetachedData(ivData, encData)
    }("Cannot detach data and IV")
  }

  private def detachDataAndGetParams(data: Array[Byte], password: Array[Char], salt: ByteVector, withIV: Boolean): EitherT[F, CryptoErr, DataWithParams] = {
    if (withIV) {
      for {
        ivDataWithEncData ← detachIV(data, IV_SIZE)
        key ← initSecretKey(password, salt.toArray)
        // setup cipher parameters with key and IV
        keyParam = new KeyParameter(key)
        params = new ParametersWithIV(keyParam, ivDataWithEncData.ivData)
      } yield DataWithParams(ivDataWithEncData.encData, params)
    } else {
      for {
        key ← initSecretKey(password, salt.toArray)
        // setup cipher parameters with key
        params = new KeyParameter(key)
      } yield DataWithParams(data, params)
    }
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
