package fluence.crypto.algorithm

import cats.data.EitherT
import cats.{ Applicative, Monad, MonadError }
import cats.syntax.applicative._
import cats.syntax.flatMap._
import fluence.codec.Codec
import fluence.crypto.algorithm.CryptoErr.nonFatalHandling
import fluence.crypto.cipher.Crypt
import fluence.crypto.facade.cryptojs.{ CryptOptions, CryptoJS, Key, KeyOptions }
import scodec.bits.ByteVector

import scalajs.js.JSConverters._
import scala.language.higherKinds
import scala.scalajs.js.typedarray.Int8Array

class AesCryptJS[F[_] : Monad, T](password: Array[Char], withIV: Boolean, config: AesConfig)(implicit ME: MonadError[F, Throwable], codec: Codec[F, T, Array[Byte]]) extends Crypt[F, T, Array[Byte]] {

  private val salt = config.salt

  private val rndStr = CryptoJS.lib.WordArray

  //number of password hashing iterations
  private val iterationCount = config.iterationCount
  //initialisation vector must be the same length as block size
  private val IV_SIZE = 16
  private val BITS = 256
  private def generateIV = rndStr.random(IV_SIZE)

  private val pad = CryptoJS.pad.Pkcs7
  private val mode = CryptoJS.mode.CBC
  private val aes = CryptoJS.AES

  override def encrypt(plainText: T): F[Array[Byte]] = {
    val e = for {
      data ← EitherT.liftF(codec.encode(plainText))
      key ← initSecretKey(new String(password), salt)
      (extData, cryptedData) = {
        val a = new Int8Array(data.toJSArray)
        val hex = CryptoJS.lib.WordArray.create(a)
        if (withIV) {
          val ivData = generateIV
          val cryptOptions = CryptOptions(iv = Some(ivData), padding = pad, mode = mode)

          val crypted = aes.encrypt(hex, key, cryptOptions)
          val ivHex = ByteVector.fromValidHex(ivData.toString)
          val cryptedHex = ByteVector.fromValidBase64(crypted.toString)
          (Some(ivHex), cryptedHex)
        } else {
          val cryptOptions = CryptOptions(iv = None, padding = pad, mode = mode)

          val crypted = aes.encrypt(hex, key, cryptOptions)
          (None, ByteVector.fromValidHex(crypted.toString))
        }
      }
    } yield extData.map(_.toArray ++ cryptedData.toArray).getOrElse(cryptedData.toArray)

    e.value.flatMap(ME.fromEither)
  }

  override def decrypt(cipherText: Array[Byte]): F[T] = {
    val dataWithParams = if (withIV) {
      val ivDec = ByteVector(cipherText.slice(0, IV_SIZE)).toHex
      val encMessage = cipherText.slice(IV_SIZE, cipherText.length)
      (Some(ivDec), encMessage)
    } else (None, cipherText)
    val (iv, data) = dataWithParams
    val base64 = ByteVector(data).toBase64
    val ivget = CryptoJS.enc.Hex.parse(iv.get)
    val e = for {
      key ← initSecretKey(new String(password), salt)
      decData = if (withIV) {
        val cryptOptions = CryptOptions(iv = Some(ivget), padding = pad, mode = mode)
        val dec = aes.decrypt(base64, key, cryptOptions)
        ByteVector.fromValidHex(dec.toString)
      } else {
        val cryptOptions = CryptOptions(iv = None, padding = pad, mode = mode)
        val dec = aes.decrypt(base64, key, cryptOptions).toString
        ByteVector.fromValidHex(dec)
      }
      plain ← EitherT.liftF[F, CryptoErr, T](codec.decode(decData.toArray))
    } yield plain

    e.value.flatMap(ME.fromEither)
  }

  private def initSecretKey(password: String, salt: String): EitherT[F, CryptoErr, Key] = {
    nonFatalHandling {
      // get raw key from password and salt
      val keyOption = KeyOptions(BITS, iterations = iterationCount, hasher = CryptoJS.algo.SHA256)
      CryptoJS.PBKDF2(password, salt, keyOption)
    }("Cannot init secret key.")
  }
}

object AesCryptJS extends slogging.LazyLogging {

  def forString[F[_] : Applicative](password: ByteVector, withIV: Boolean, config: AesConfig)(implicit ME: MonadError[F, Throwable]): AesCryptJS[F, String] = {
    implicit val codec: Codec[F, String, Array[Byte]] = Codec[F, String, Array[Byte]](_.getBytes.pure[F], bytes ⇒ new String(bytes).pure[F])
    apply[F, String](password, withIV, config)
  }

  def apply[F[_] : Applicative, T](password: ByteVector, withIV: Boolean, config: AesConfig)(implicit ME: MonadError[F, Throwable], codec: Codec[F, T, Array[Byte]]): AesCryptJS[F, T] =
    new AesCryptJS(password.toHex.toCharArray, withIV, config)
}
