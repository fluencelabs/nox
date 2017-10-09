package fluence.core

import freestyle._
import freestyle.implicits._

@free trait Interaction {
  def tell(msg: String): FS[Unit]

  def ask(prompt: String): FS[String]
}
