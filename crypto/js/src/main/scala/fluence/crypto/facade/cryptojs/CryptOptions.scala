package fluence.crypto.facade.cryptojs

import scala.scalajs.js

@js.native
trait CryptOptions extends js.Object {
  val iv: Option[js.Any]
  val padding: Pad
  val mode: Mode
}

object CryptOptions {
  def apply(iv: Option[js.Any], padding: Pad, mode: Mode): CryptOptions = {
    iv match {
      case Some(i) ⇒
        js.Dynamic.literal(iv = i, padding = padding, mode = mode).asInstanceOf[CryptOptions]
      case None ⇒
        js.Dynamic.literal(padding = padding, mode = mode).asInstanceOf[CryptOptions]
    }

  }
}
