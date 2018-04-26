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

import cats.data.EitherT
import cats.Monad
import cats.syntax.compose._
import fluence.codec.PureCodec
import fluence.crypto.{Crypto, CryptoError}
import fluence.crypto.CryptoError.nonFatalHandling
import fluence.crypto.facade.cryptojs.{CryptOptions, CryptoJS, Key, KeyOptions}
import scodec.bits.ByteVector

import scalajs.js.JSConverters._
import scala.language.higherKinds
import scala.scalajs.js.typedarray.Int8Array

class AesCrypt(password: Array[Char], withIV: Boolean, config: AesConfig) {

  private val salt = config.salt

  private val rndStr = CryptoJS.lib.WordArray

  //number of password hashing iterations
  private val iterationCount = config.iterationCount
  //initialisation vector must be the same length as block size
  private val IV_SIZE = 16
  private val BITS = 256
  //generate IV in hex
  private def generateIV = rndStr.random(IV_SIZE)

  private val pad = CryptoJS.pad.Pkcs7
  private val mode = CryptoJS.mode.CBC
  private val aes = CryptoJS.AES

  val encrypt: Crypto.Func[Array[Byte], Array[Byte]] =
    new Crypto.Func[Array[Byte], Array[Byte]] {
      override def apply[F[_]: Monad](input: Array[Byte]): EitherT[F, CryptoError, Array[Byte]] =
        for {
          key ← initSecretKey()
          encrypted ← encryptData(input, key)
        } yield encrypted
    }

  val decrypt: Crypto.Func[Array[Byte], Array[Byte]] =
    new Crypto.Func[Array[Byte], Array[Byte]] {
      override def apply[F[_]: Monad](input: Array[Byte]): EitherT[F, CryptoError, Array[Byte]] =
        for {
          detachedData ← detachData(input)
          (iv, base64) = detachedData
          key ← initSecretKey()
          decData ← decryptData(key, base64, iv)
          _ ← EitherT.cond(decData.nonEmpty, decData, CryptoError("Cannot decrypt message with this password."))
        } yield decData.toArray
    }

  /**
   * Encrypt data.
   * @param data Data to encrypt
   * @param key Salted and hashed password
   * @return Encrypted data with IV
   */
  private def encryptData[F[_]: Monad](data: Array[Byte], key: Key): EitherT[F, CryptoError, Array[Byte]] = {
    nonFatalHandling {
      //transform data to JS type
      val wordArray = CryptoJS.lib.WordArray.create(new Int8Array(data.toJSArray))
      val iv = if (withIV) Some(generateIV) else None
      val cryptOptions = CryptOptions(iv = iv, padding = pad, mode = mode)
      //encryption return base64 string, transform it to byte array
      val crypted = ByteVector.fromValidBase64(aes.encrypt(wordArray, key, cryptOptions).toString)
      //IV also needs to be transformed in byte array
      val byteIv = iv.map(i ⇒ ByteVector.fromValidHex(i.toString))
      byteIv.map(_.toArray ++ crypted.toArray).getOrElse(crypted.toArray)
    }("Cannot encrypt data.")
  }

  private def decryptData[F[_]: Monad](key: Key, base64Data: String, iv: Option[String]) = {
    nonFatalHandling {
      //parse IV to WordArray JS format
      val cryptOptions = CryptOptions(iv = iv.map(i ⇒ CryptoJS.enc.Hex.parse(i)), padding = pad, mode = mode)
      val dec = aes.decrypt(base64Data, key, cryptOptions)
      ByteVector.fromValidHex(dec.toString)
    }("Cannot decrypt data.")
  }

  /**
   * @param cipherText Encrypted data with IV
   * @return IV in hex and data in base64
   */
  private def detachData[F[_]: Monad](cipherText: Array[Byte]): EitherT[F, CryptoError, (Option[String], String)] = {
    nonFatalHandling {
      val dataWithParams = if (withIV) {
        val ivDec = ByteVector(cipherText.slice(0, IV_SIZE)).toHex
        val encMessage = cipherText.slice(IV_SIZE, cipherText.length)
        (Some(ivDec), encMessage)
      } else (None, cipherText)
      val (ivOp, data) = dataWithParams
      val base64 = ByteVector(data).toBase64
      (ivOp, base64)
    }("Cannot detach data and IV.")
  }

  /**
   * Hash password with salt `iterationCount` times
   */
  private def initSecretKey[F[_]: Monad](): EitherT[F, CryptoError, Key] = {
    nonFatalHandling {
      // get raw key from password and salt
      val keyOption = KeyOptions(BITS, iterations = iterationCount, hasher = CryptoJS.algo.SHA256)
      CryptoJS.PBKDF2(new String(password), salt, keyOption)
    }("Cannot init secret key.")
  }
}

object AesCrypt extends slogging.LazyLogging {

  def build(password: ByteVector, withIV: Boolean, config: AesConfig): Crypto.Cipher[Array[Byte]] = {
    val aes = new AesCrypt(password.toHex.toCharArray, withIV, config)
    Crypto.Bijection(aes.encrypt, aes.decrypt)
  }

  def forString(password: ByteVector, withIV: Boolean, config: AesConfig): Crypto.Cipher[String] = {
    implicit val codec: PureCodec[String, Array[Byte]] =
      PureCodec.build(_.getBytes, bytes ⇒ new String(bytes))
    apply[String](password, withIV, config)
  }

  def apply[T](password: ByteVector, withIV: Boolean, config: AesConfig)(
    implicit codec: PureCodec[T, Array[Byte]]
  ): Crypto.Cipher[T] =
    Crypto.codec[T, Array[Byte]] andThen build(password, withIV, config)
}
