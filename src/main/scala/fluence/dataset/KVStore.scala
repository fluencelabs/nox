package fluence.dataset

import freestyle._
import freestyle.implicits._

@free trait KVStore {
  def get(key: Array[Byte]): FS[Option[Array[Byte]]]

  def put(key: Array[Byte], value: Array[Byte]): FS[Boolean]

  def contains(key: Array[Byte]): FS[Boolean] = get(key).map(_.isDefined)

  def remove(key: Array[Byte]): FS[Boolean]
}
