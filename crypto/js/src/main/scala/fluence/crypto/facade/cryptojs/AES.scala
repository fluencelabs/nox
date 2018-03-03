package fluence.crypto.facade.cryptojs

import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal

@js.native
@JSGlobal
class AES extends js.Object {

  /**
   * @param msg Message to encrypt in JS WordArray.
   *            Could be created with CryptoJS.lib.WordArray.create(new Int8Array(arrayByte.toJSArray))
   * @param options { iv: iv, padding: CryptoJS.pad.Pkcs7, mode: CryptoJS.mode.CBC }
   * @return Encrypted message
   */
  def encrypt(msg: WordArray, key: Key, options: CryptOptions): js.Any = js.native

  def decrypt(encrypted: String, key: Key, options: CryptOptions): js.Any = js.native
}
