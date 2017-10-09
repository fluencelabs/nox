package fluence.dataset

import freestyle._
import freestyle.implicits._

@module trait ServerModule {
  val kvStore: KVStore
}
