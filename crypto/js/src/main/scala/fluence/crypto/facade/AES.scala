package fluence.crypto.facade

import scala.scalajs.js
import scala.scalajs.js.annotation.{ JSGlobal, JSImport }

@js.native
@JSGlobal
class AES extends js.Object {

  /**
   *
   * @param msg
   * @param options { iv: iv, padding: CryptoJS.pad.Pkcs7, mode: CryptoJS.mode.CBC }
   * @return
   */
  def encrypt(msg: js.Any, key: Key, options: js.Dynamic): js.Any = js.native

  def decrypt(encrypted: String, key: Key, options: js.Dynamic): js.Any = js.native
}
