package fluence.crypto

import fluence.crypto.algorithm.{ AesConfig, AesCryptJS }
import cats.instances.try_._
import fluence.codec.Codec
import scodec.bits.ByteVector

import scala.util.Try

object TestApp {

  def main(args: Array[String]): Unit = {
    implicit val codec: Codec[Try, String, Array[Byte]] = Codec[Try, String, Array[Byte]](b ⇒ Try(b.getBytes), bytes ⇒ Try(new String(bytes)))
    val crypt = AesCryptJS.forString[Try](ByteVector("somepass".getBytes), true, AesConfig(salt = "salt"))

    val encrypted1 = crypt.encrypt("some message").get
    println("ENCRYPTED ==== " + ByteVector(encrypted1).toHex)

    val decrypted1 = crypt.decrypt(encrypted1).get
    println("DECRYPTED ==== " + decrypted1)
  }

}
