package fluence.crypto.facade.cryptojs

import scala.scalajs.js

@js.native
trait WordArray extends js.Object {

  def random(size: Int): js.Any = js.native
  def create(array: js.Any): js.Any = js.native
}
