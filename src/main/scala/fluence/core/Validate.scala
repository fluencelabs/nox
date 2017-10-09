package fluence.core

import freestyle._
import freestyle.implicits._

@free trait Validate {
  def minSize(s: String, n: Int): FS[Boolean]

  def hasNumber(s: String): FS[Boolean]
}
