package fluence.crypto.facade

import scala.scalajs.js
import scala.scalajs.js.annotation.{ JSGlobal, JSImport }

@js.native
@JSImport("crypto-js", JSImport.Namespace)
object CryptoJS extends js.Object {

  def pad: Pad = js.native
  def mode: Mode = js.native
  def AES: AES = js.native

  def PBKDF2(pass: js.Any, salt: String, options: js.Dynamic): Key = js.native

  def lib: Lib = js.native

  def enc: Enc = js.native

  def SHA256(str: String): js.Any = js.native

  def algo: Algo = js.native
}
