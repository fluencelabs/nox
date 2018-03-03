package fluence.crypto.facade.cryptojs

import scala.scalajs.js

@js.native
trait Hex extends js.Object {
  /**
   * Parse from HEX to JS byte representation
   * @param str Hex
   */
  def parse(str: String): WordArray = js.native
}
