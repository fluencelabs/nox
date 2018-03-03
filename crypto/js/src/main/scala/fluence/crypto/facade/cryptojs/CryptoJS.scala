package fluence.crypto.facade.cryptojs

import scala.scalajs.js
import scala.scalajs.js.annotation.JSImport

@js.native
@JSImport("crypto-js", JSImport.Namespace)
object CryptoJS extends js.Object {

  def pad: Paddings = js.native
  def mode: Modes = js.native
  def AES: AES = js.native

  /**
   * https://en.wikipedia.org/wiki/PBKDF2
   * @return Salted and hashed key
   */
  def PBKDF2(pass: String, salt: String, options: KeyOptions): Key = js.native

  def lib: Lib = js.native

  def enc: Enc = js.native

  def algo: Algos = js.native
}
