package fluence.crypto

import fluence.crypto.facade.{ AES, CryptoJS }

import scala.scalajs.js

object TestApp {

  def main(args: Array[String]): Unit = {

    println("ALO")

    val cryptoJS = CryptoJS

    val aes = AES

    val pad = cryptoJS.pad.PBKDF2

    println("AES === " + aes)
    println("PAD === " + pad)

    //  val options = js.Dynamic.literal(iv = "123", padding = CryptoJS.pad.Pkcs7, mode = CryptoJS.mode.CBC)

    //  aes.encrypt("msg", 256, )
  }

}
