package fluence.crypto.facade.cryptojs

import scala.scalajs.js

@js.native
trait WordArrayFactory extends js.Object {

  def random(size: Int): WordArray = js.native
  def create(array: js.Any): WordArray = js.native
}

@js.native
trait WordArray extends js.Object
