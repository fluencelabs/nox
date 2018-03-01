package fluence.crypto.algorithm

import cats.data.EitherT
import cats.{ Applicative, Monad, MonadError }
import cats.syntax.applicative._
import cats.syntax.flatMap._
import fluence.codec.Codec
import fluence.crypto.algorithm.CryptoErr.nonFatalHandling
import fluence.crypto.cipher.Crypt
import fluence.crypto.facade.{ CryptoJS, Key }
import scodec.bits.ByteVector

import scalajs.js.JSConverters._
import scala.language.higherKinds
import scala.scalajs.js
import scala.scalajs.js.typedarray.Int8Array

class AesCryptJS[F[_] : Monad, T](password: Array[Char], withIV: Boolean, config: AesConfig)(implicit ME: MonadError[F, Throwable], codec: Codec[F, T, Array[Byte]]) extends Crypt[F, T, Array[Byte]] {

  private val salt = config.salt

  private val rndStr = CryptoJS.lib.WordArray

  //number of password hashing iterations
  //todo should be configurable
  private val iterationCount = config.iterationCount
  //initialisation vector must be the same length as block size
  private val IV_SIZE = 16
  private val BITS = 256
  private def generateIV = rndStr.random(IV_SIZE)

  def pad = CryptoJS.pad.Pkcs7
  def mode = CryptoJS.mode.CBC
  val aes = CryptoJS.AES

  override def encrypt(plainText: T): F[Array[Byte]] = {
    val e = for {
      data ← EitherT.liftF(codec.encode(plainText))
      key ← initSecretKey(password.toString, salt)
      (extData, cryptedData) = {
        if (withIV) {
          val ivData = generateIV
          println("KEY === " + key)
          println("IV DATA === " + ivData)
          println("KEY IN AESCRYPT === " + key)
          val cryptOptions = js.Dynamic.literal(iv = ivData, padding = pad, mode = mode)
          //          println("CRYPT OPTIONS === " + cryptOptions)

          val a = new Int8Array(data.toJSArray)
          val hex = CryptoJS.lib.WordArray.create(a)
          println("HEX === " + hex)
          val crypted = aes.encrypt(hex, key, cryptOptions)
          println("CRYPTED IN AESCRYPT === " + crypted)
          val ivHex = ByteVector.fromValidHex(ivData.toString)
          println("IV HEX === " + ivHex)
          println("CRYPTED NEHEX === " + crypted.toString)
          val cryptedHex = ByteVector.fromValidBase64(crypted.toString)
          println("CRYPTED HEX === " + cryptedHex)
          (Some(ivHex), cryptedHex)
        } else {
          val cryptOptions = js.Dynamic.literal(padding = pad, mode = mode)
          val a = new Int8Array(data.toJSArray)
          val hex = CryptoJS.lib.WordArray.create(a)
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
    println("BASE64 BEFORE DECRYPT === " + base64)
    println("IV DEC === " + iv)
    val e = for {
      key ← initSecretKey(password.toString, salt)
      _ = println("KEY === " + key)
      decData = if (withIV) {
        val cryptOptions = js.Dynamic.literal(iv = ivget, padding = pad, mode = mode)
        val dec = aes.decrypt(base64, key, cryptOptions)
        println("DEC ===" + dec)
        ByteVector.fromValidHex(dec.toString)
      } else {
        val cryptOptions = js.Dynamic.literal(padding = pad, mode = mode)
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
      val keySize = BITS / 32
      println("KEY SIZE === " + keySize)
      println("ITERATION COUNT === " + iterationCount)
      val keyOption = js.Dynamic.literal(keySize = keySize, iterations = iterationCount)
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
