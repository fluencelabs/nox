/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

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
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.util.Random

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
 */
class AesCrypt[F[_] : Monad, T](password: Array[Char], withIV: Boolean, config: AesConfig)(serializer: T ⇒ F[Array[Byte]], deserializer: Array[Byte] ⇒ F[T])(implicit ME: MonadError[F, Throwable])
  extends Crypt[F, T, Array[Byte]] with JavaAlgorithm {
  import CryptoErr._

  private val rnd = Random
  private val salt = config.salt.getBytes()

  //number of password hashing iterations
  //todo should be configurable
  private val iterationCount = config.iterationCount
  //initialisation vector must be the same length as block size
  private val IV_SIZE = 16
  private val BITS = 256
  private def generateIV: Array[Byte] = {
    val iv = new Array[Byte](IV_SIZE)
    rnd.nextBytes(iv)
    iv
  }

  override def encrypt(plainText: T): F[Array[Byte]] = {
    val e = for {
      data ← EitherT.liftF(serializer(plainText))
      key ← initSecretKey(password, salt)
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
      val pbeKeySpec = new PBEKeySpec(password, salt, iterationCount, BITS)
      val keyFactory: SecretKeyFactory = SecretKeyFactory.getInstance("PBEWithSHA256And256BitAES-CBC-BC")
      val secretKey = new SecretKeySpec(keyFactory.generateSecret(pbeKeySpec).getEncoded, "AES")
      secretKey.getEncoded
    }("Cannot init secret key.")
  }

  /**
   * Setup AES CBC cipher
   * @param encrypt True for encryption and false for decryption
   * @return cipher
   */
  private def setupAesCipher(params: CipherParameters, encrypt: Boolean): EitherT[F, CryptoErr, PaddedBufferedBlockCipher] = {
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
    }("Error in cipher processing.")
  }

  /**
   *
   * @param dataWithParams Cata with cipher parameters
   * @param addData Additional data (nonce)
   * @param encrypt True for encryption and false for decryption
   * @return Crypted bytes
   */
  private def processData(dataWithParams: DataWithParams, addData: Option[Array[Byte]], encrypt: Boolean): EitherT[F, CryptoErr, Array[Byte]] = {
    for {
      cipher ← setupAesCipher(dataWithParams.params, encrypt = encrypt)
      buf ← cipherBytes(dataWithParams.data, cipher)
      encryptedData = addData.map(_ ++ buf).getOrElse(buf)
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
    }("Cannot detach data and IV.")
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

  def forString[F[_] : Applicative](password: ByteVector, withIV: Boolean, config: AesConfig)(implicit ME: MonadError[F, Throwable]): AesCrypt[F, String] =
    apply[F, String](password, withIV, config)(
      serializer = _.getBytes.pure[F],
      deserializer = bytes ⇒ new String(bytes).pure[F]
    )

  def apply[F[_] : Applicative, T](password: ByteVector, withIV: Boolean, config: AesConfig)(serializer: T ⇒ F[Array[Byte]], deserializer: Array[Byte] ⇒ F[T])(implicit ME: MonadError[F, Throwable]): AesCrypt[F, T] =
    new AesCrypt(password.toHex.toCharArray, withIV, config)(serializer, deserializer)

}
