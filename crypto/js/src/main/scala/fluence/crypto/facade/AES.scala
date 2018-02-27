package fluence.crypto.facade

import scala.scalajs.js
import scala.scalajs.js.annotation.JSImport

@js.native
@JSImport("crypto-js/aes", "AES")
object AES extends js.Object {

  /**
   *
   * @param msg
   * @param keySize Bits size, i.e. 256
   * @param options { iv: iv, padding: CryptoJS.pad.Pkcs7, mode: CryptoJS.mode.CBC }
   * @return
   */
  def encrypt(msg: String, keySize: Int, options: js.Dynamic): js.Any = js.native
}
