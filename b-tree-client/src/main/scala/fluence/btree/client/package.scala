package fluence.btree

import cats.Show

package object client {

  /* Cipher key */
  type Key = Array[Byte]
  /* Cipher value */
  type Value = Array[Byte]

  type Bytes = Array[Byte]

  implicit private[client] val showBytes: Show[Key] =
    (b: Key) ⇒ new String(b)

}
