package fluence.crypto.facade.cryptojs

import scala.scalajs.js

@js.native
trait CryptOptions extends js.Object {
  val iv: Option[js.Any]
  val padding: Pad
  val mode: Mode
}

object CryptOptions {
  def apply(iv: Option[WordArray], padding: Pad, mode: Mode): CryptOptions = {
    iv match {
      case Some(i) ⇒
        js.Dynamic.literal(iv = i, padding = padding, mode = mode).asInstanceOf[CryptOptions]
      case None ⇒
        //if IV is empty, there will be an error in JS lib
        js.Dynamic.literal(iv = CryptoJS.lib.WordArray.random(0), padding = padding, mode = mode).asInstanceOf[CryptOptions]
    }

  }
}
