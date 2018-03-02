package fluence.crypto.facade.cryptojs

import scala.scalajs.js

@js.native
trait Algos extends js.Object {
  def SHA256: Algo = js.native
}

@js.native
trait Algo extends js.Object
