package fluence.crypto

import fluence.crypto.algorithm.{ AesConfig, AesCryptJS }
import fluence.crypto.facade.{ AES, CryptoJS }
import cats.instances.try_._
import fluence.codec.Codec
import scodec.bits.ByteVector

import scalajs.js.JSConverters._
import scala.scalajs.js
import scala.scalajs.js.typedarray.Int8Array
import scala.util.Try

object TestApp {

  def main(args: Array[String]): Unit = {

    def a() = {
      println("ALO")

      val aes = CryptoJS.AES

      val pad = CryptoJS.pad.Pkcs7
      val mode = CryptoJS.mode.CBC

      println("AES === " + aes)
      println("MODE === " + mode)
      println("PAD === " + pad)

      val salt = "salt"

      val keyOption = js.Dynamic.literal(keySize = 256 / 32, iterations = 50)
      val key = CryptoJS.PBKDF2("password", salt, keyOption)

      println("KEY === " + key)

      val iv = CryptoJS.lib.WordArray.random(128 / 8)
      println("IV === " + iv)

      val message = new Int8Array("some message".getBytes().toJSArray)
      val message2 = "some message"
      val cryptOptions = js.Dynamic.literal(iv = iv, padding = pad, mode = mode)
      val crypted = aes.encrypt(message2, key, cryptOptions)

      println("CRYPTED === " + crypted)

      val transitmessage = iv.toString + crypted.toString

      println("TRANSIT MESSAGES === " + transitmessage)

      val ivDec = CryptoJS.enc.Hex.parse(transitmessage.substring(0, 32))
      val encMessage = transitmessage.substring(32)

      println("ENC MESSAGE === " + encMessage)

      val keyOption2 = js.Dynamic.literal(keySize = 256 / 32, iterations = 50)
      val key2 = CryptoJS.PBKDF2("password", salt, keyOption2)

      val cryptOptions2 = js.Dynamic.literal(iv = ivDec, padding = pad, mode = mode)
      val bytes = CryptoJS.AES.decrypt(encMessage, key2, cryptOptions2)

      println("DECRYPTED === " + bytes)
      println("DECRYPTED === " + bytes.toString)
      println("DECRYPTED === " + CryptoJS.enc.Hex.parse(bytes.toString))

      println("DECRYPTED STRING === " + CryptoJS.enc.Utf8.stringify(bytes))
    }

    a()
    a()

    //  val options = js.Dynamic.literal(iv = "123", padding = CryptoJS.pad.Pkcs7, mode = CryptoJS.mode.CBC)

    //  aes.encrypt("msg", 256, )
    implicit val codec: Codec[Try, String, Array[Byte]] = Codec[Try, String, Array[Byte]](b ⇒ Try(b.getBytes), bytes ⇒ Try(new String(bytes)))
    val crypt = AesCryptJS.forString[Try](ByteVector("somepass".getBytes), true, AesConfig(salt = "salt"))

    val encrypted1 = crypt.encrypt("some message").get
    println("ENCRYPTED ==== " + ByteVector(encrypted1).toHex)

    val decrypted1 = crypt.decrypt(encrypted1).get
    println("DECRYPTED ==== " + decrypted1)
  }

}
