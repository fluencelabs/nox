package fluence.core

import freestyle._
import freestyle.implicits._

@module trait AppModule {
  val interaction: Interaction
  val validation: Validate
}
