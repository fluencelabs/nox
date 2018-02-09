package fluence.crypto.facade

import scala.scalajs.js
import js.annotation._
import scala.scalajs.js.annotation.JSImport.Namespace
import js.|
import scala.scalajs.js.typedarray.{ Int8Array, Uint8Array }

class Elliptic {

}

@js.native
@JSImport("elliptic", "ec")
class EC(curve: String) extends js.Object {
  def genKeyPair(options: Option[js.Dynamic] = None): KeyPair = js.native
  def keyPair(options: js.Dynamic): KeyPair = js.native
  def keyFromPublic(pub: String, enc: String): KeyPair = js.native
  def keyFromPrivate(priv: String, enc: String): KeyPair = js.native
}

@js.native
@JSImport("elliptic", "ec")
class KeyPair(ec: EC, options: js.Dynamic) extends js.Object {
  def verify(msg: js.Array[Byte], signature: String): Boolean = js.native
  def sign(msg: js.Array[Byte]): Signature = js.native

  def getPublic(compact: Boolean, enc: String): String = js.native
  def getPrivate(enc: String): String = js.native
  val priv: js.Any = js.native
  val pub: js.Any = js.native
}

@js.native
@JSImport("elliptic", "ec")
class Signature(der: String, enc: String = "hex") extends js.Object {
  def toDER(enc: String): String = js.native
}
