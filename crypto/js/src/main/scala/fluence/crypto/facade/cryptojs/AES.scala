package fluence.crypto.facade.cryptojs

import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal

@js.native
@JSGlobal
class AES extends js.Object {

  /**
   *
   * @param msg
   * @param options { iv: iv, padding: CryptoJS.pad.Pkcs7, mode: CryptoJS.mode.CBC }
   * @return
   */
  def encrypt(msg: js.Any, key: Key, options: CryptOptions): js.Any = js.native

  def decrypt(encrypted: String, key: Key, options: CryptOptions): js.Any = js.native
}
