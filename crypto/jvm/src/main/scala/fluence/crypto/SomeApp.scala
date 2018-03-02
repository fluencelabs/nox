package fluence.crypto

import fluence.codec.Codec
import fluence.crypto.algorithm.{ AesConfig, AesCrypt }
import scodec.bits.ByteVector
import cats.instances.try_._

import scala.util.Try

object SomeApp extends App {
  implicit val codec: Codec[Try, String, Array[Byte]] = Codec[Try, String, Array[Byte]](b ⇒ Try(b.getBytes), bytes ⇒ Try(new String(bytes)))
  val crypt = AesCrypt.forString[Try](ByteVector("somepass".getBytes), true, AesConfig(salt = "salt"))

  val encrypted1 = crypt.encrypt("some message").get
  println("ENCRYPTED ==== " + ByteVector(encrypted1).toHex)

  val decrypted1 = crypt.decrypt(encrypted1).get
  println("DECRYPTED ==== " + decrypted1)
}
