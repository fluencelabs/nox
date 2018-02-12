package fluence.crypto.facade

import scala.scalajs.js
import scala.scalajs.js.annotation.JSImport

@js.native
@JSImport("hash.js", "sha256")
class SHA256() extends js.Object {
  def update(msg: js.Array[Byte]): Unit = js.native
  def digest(enc: String): String = js.native
}
