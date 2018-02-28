package fluence.crypto

import fluence.crypto.facade.{ AES, CryptoJS }

import scala.scalajs.js

object TestApp {

  def main(args: Array[String]): Unit = {

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
    println("RANDOM === " + iv)

    val message = "some message"
    val cryptOptions = js.Dynamic.literal(iv = iv, padding = pad, mode = mode)
    val crypted = aes.encrypt(message, key, cryptOptions)

    println("CRYPTED === " + crypted)

    val transitmessage = iv.toString + crypted.toString

    println("TRANSIT MESSAGES === " + transitmessage)

    var ivDec = CryptoJS.enc.Hex.parse(transitmessage.substring(0, 32))
    val encMessage = transitmessage.substring(32)

    val keyOption2 = js.Dynamic.literal(keySize = 256 / 32, iterations = 50)
    val key2 = CryptoJS.PBKDF2("password", salt, keyOption2)

    val cryptOptions2 = js.Dynamic.literal(iv = ivDec, padding = pad, mode = mode)
    val bytes = CryptoJS.AES.decrypt(encMessage, key2, cryptOptions2)

    println("DECRYPTED === " + bytes)
    println("DECRYPTED === " + bytes.toString)
    println("DECRYPTED === " + CryptoJS.enc.Hex.parse(bytes.toString))

    println("DECRYPTED STRING === " + CryptoJS.enc.Utf8.stringify(bytes))

    //  val options = js.Dynamic.literal(iv = "123", padding = CryptoJS.pad.Pkcs7, mode = CryptoJS.mode.CBC)

    //  aes.encrypt("msg", 256, )
  }

}
