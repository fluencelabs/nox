package fluence.crypto.facade.cryptojs

import scala.scalajs.js

@js.native
trait KeyOptions extends js.Object {
  val keySize: Int
  val iterations: Int
  val hasher: Algo
}

object KeyOptions {
  def apply(keySizeBits: Int, iterations: Int, hasher: Algo): KeyOptions = {
    js.Dynamic.literal(keySize = keySizeBits / 32, iterations = iterations, hasher = hasher).asInstanceOf[KeyOptions]
  }
}
