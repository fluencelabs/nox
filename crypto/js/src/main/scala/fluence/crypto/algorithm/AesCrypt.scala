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
import cats.{Applicative, Monad, MonadError}
import cats.syntax.applicative._
import cats.syntax.flatMap._
import fluence.codec.Codec
import fluence.crypto.algorithm.CryptoErr.nonFatalHandling
import fluence.crypto.cipher.Crypt
import fluence.crypto.facade.cryptojs.{CryptOptions, CryptoJS, Key, KeyOptions}
import scodec.bits.ByteVector

import scalajs.js.JSConverters._
import scala.language.higherKinds
import scala.scalajs.js.typedarray.Int8Array

class AesCrypt[F[_]: Monad, T](password: Array[Char], withIV: Boolean, config: AesConfig)(
  implicit ME: MonadError[F, Throwable],
  codec: Codec[F, T, Array[Byte]]
) extends Crypt[F, T, Array[Byte]] {

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

  override def encrypt(plainText: T): F[Array[Byte]] = {
    val e = for {
      data ← EitherT.liftF(codec.encode(plainText))
      key ← initSecretKey()
      encrypted ← encryptData(data, key)
    } yield encrypted

    e.value.flatMap(ME.fromEither)
  }

  override def decrypt(cipherText: Array[Byte]): F[T] = {
    val e = for {
      detachedData ← detachData(cipherText)
      (iv, base64) = detachedData
      key ← initSecretKey()
      decData ← decryptData(key, base64, iv)
      _ ← EitherT.cond(decData.nonEmpty, decData, CryptoErr("Cannot decrypt message with this password."))
      plain ← EitherT.liftF[F, CryptoErr, T](codec.decode(decData.toArray))
    } yield plain

    e.value.flatMap(ME.fromEither)
  }

  /**
   * Encrypt data.
   * @param data Data to encrypt
   * @param key Salted and hashed password
   * @return Encrypted data with IV
   */
  private def encryptData(data: Array[Byte], key: Key): EitherT[F, CryptoErr, Array[Byte]] = {
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

  private def decryptData(key: Key, base64Data: String, iv: Option[String]) = {
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
  private def detachData(cipherText: Array[Byte]): EitherT[F, CryptoErr, (Option[String], String)] = {
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
  private def initSecretKey(): EitherT[F, CryptoErr, Key] = {
    nonFatalHandling {
      // get raw key from password and salt
      val keyOption = KeyOptions(BITS, iterations = iterationCount, hasher = CryptoJS.algo.SHA256)
      CryptoJS.PBKDF2(new String(password), salt, keyOption)
    }("Cannot init secret key.")
  }
}

object AesCrypt extends slogging.LazyLogging {

  def forString[F[_]: Applicative](password: ByteVector, withIV: Boolean, config: AesConfig)(
    implicit ME: MonadError[F, Throwable]
  ): AesCrypt[F, String] = {
    implicit val codec: Codec[F, String, Array[Byte]] =
      Codec[F, String, Array[Byte]](_.getBytes.pure[F], bytes ⇒ new String(bytes).pure[F])
    apply[F, String](password, withIV, config)
  }

  def apply[F[_]: Applicative, T](password: ByteVector, withIV: Boolean, config: AesConfig)(
    implicit ME: MonadError[F, Throwable],
    codec: Codec[F, T, Array[Byte]]
  ): AesCrypt[F, T] =
    new AesCrypt(password.toHex.toCharArray, withIV, config)
}
