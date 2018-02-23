package fluence.crypto.algorithm

import java.security.SecureRandom

import cats.{ Applicative, Monad, MonadError }
import cats.data.EitherT
import cats.syntax.flatMap._
import cats.syntax.applicative._
import fluence.crypto.cipher.Crypt
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

import scala.language.higherKinds

case class DetachedData(ivData: Array[Byte], encData: Array[Byte])
case class DataWithParams(data: Array[Byte], params: CipherParameters)

/**
 * PBEWithSHA256And256BitAES-CBC-BC cryptography
 * PBE - Password-based encryption
 * SHA256 - hash for password
 * AES with CBC BC - Advanced Encryption Standard with Cipher Block Chaining
 * https://ru.wikipedia.org/wiki/Advanced_Encryption_Standard
 * https://en.wikipedia.org/wiki/Block_cipher_mode_of_operation#Cipher_Block_Chaining_(CBC)
 * @param password User entered password
 * @param withIV Initialization vector to achieve semantic security, a property whereby repeated usage of the scheme
 *               under the same key does not allow an attacker to infer relationships between segments of the encrypted
 *               message
 * @param salt Salt for password
 */
class AesCrypt[F[_] : Monad, T](password: Array[Char], withIV: Boolean, salt: Array[Byte])(serializer: T ⇒ F[Array[Byte]], deserializer: Array[Byte] ⇒ F[T])(implicit ME: MonadError[F, Throwable])
  extends Crypt[F, T, Array[Byte]] with JavaAlgorithm {
  import CryptoErr._

  private val rnd = new SecureRandom()
  private val BITS = 256

  //number of password hashing iterations
  //todo should be configurable
  private val ITERATION_COUNT = 50
  //initialisation vector must be the same length as block size
  private val IV_SIZE = 16
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

  /**
   * key spec initialization
   */
  private def initSecretKey(password: Array[Char], salt: Array[Byte]): EitherT[F, CryptoErr, Array[Byte]] = {
    nonFatalHandling {
      // get raw key from password and salt
      val pbeKeySpec = new PBEKeySpec(password, salt, ITERATION_COUNT, BITS)
      val keyFactory: SecretKeyFactory = SecretKeyFactory.getInstance("PBEWithSHA256And256BitAES-CBC-BC")
      val secretKey = new SecretKeySpec(keyFactory.generateSecret(pbeKeySpec).getEncoded, "AES")
      secretKey.getEncoded
    }("Cannot init secret key.")
  }

  /**
   * Setup AES CBC cipher
   * @param encrypt True for encryption and false for decryption
   * @return
   */
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

  private def cipherBytes(data: Array[Byte], cipher: PaddedBufferedBlockCipher): EitherT[F, CryptoErr, Array[Byte]] = {
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
      buf ← cipherBytes(dataWithParams.data, cipher)
      encryptedData = extData.map(_ ++ buf).getOrElse(buf)
    } yield encryptedData
  }

  /**
   * encrypted data = initialization vector + data
   */
  private def detachIV(data: Array[Byte], ivSize: Int): EitherT[F, CryptoErr, DetachedData] = {
    nonFatalHandling {
      val ivData = data.slice(0, ivSize)
      val encData = data.slice(ivSize, data.length)
      DetachedData(ivData, encData)
    }("Cannot detach data and IV")
  }

  private def detachDataAndGetParams(data: Array[Byte], password: Array[Char], salt: Array[Byte], withIV: Boolean): EitherT[F, CryptoErr, DataWithParams] = {
    if (withIV) {
      for {
        ivDataWithEncData ← detachIV(data, IV_SIZE)
        key ← initSecretKey(password, salt)
        // setup cipher parameters with key and IV
        keyParam = new KeyParameter(key)
        params = new ParametersWithIV(keyParam, ivDataWithEncData.ivData)
      } yield DataWithParams(ivDataWithEncData.encData, params)
    } else {
      for {
        key ← initSecretKey(password, salt)
        // setup cipher parameters with key
        params = new KeyParameter(key)
      } yield DataWithParams(data, params)
    }
  }
}

object AesCrypt extends slogging.LazyLogging {

  val fluenceSalt: Array[Byte] = "fluence".getBytes()

  def forString[F[_] : Applicative](password: Array[Char], withIV: Boolean, salt: Array[Byte] = fluenceSalt)(implicit ME: MonadError[F, Throwable]): AesCrypt[F, String] =
    apply[F, String](password, withIV, salt)(
      serializer = _.getBytes.pure[F],
      deserializer = bytes ⇒ new String(bytes).pure[F]
    )

  def apply[F[_] : Applicative, T](password: Array[Char], withIV: Boolean, salt: Array[Byte] = fluenceSalt)(serializer: T ⇒ F[Array[Byte]], deserializer: Array[Byte] ⇒ F[T])(implicit ME: MonadError[F, Throwable]): AesCrypt[F, T] =
    new AesCrypt(password, withIV, salt)(serializer, deserializer)

}
