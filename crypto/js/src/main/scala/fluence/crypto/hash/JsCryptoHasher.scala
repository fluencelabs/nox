package fluence.crypto.hash

import fluence.crypto.facade.SHA256
import scodec.bits.ByteVector

import scala.scalajs.js.JSConverters._

object JsCryptoHasher {

  lazy val Sha256: CryptoHasher[Array[Byte], Array[Byte]] = new CryptoHasher[Array[Byte], Array[Byte]] {

    override def hash(msg1: Array[Byte]): Array[Byte] = {
      val sha256 = new SHA256()
      sha256.update(msg1.toJSArray)
      ByteVector.fromHex(sha256.digest("hex")).get.toArray
    }

    override def hash(msg1: Array[Byte], msg2: Array[Byte]*): Array[Byte] = {
      hash(msg1 ++ msg2.flatten)
    }

  }
}
