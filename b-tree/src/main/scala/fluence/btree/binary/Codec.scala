package fluence.btree.binary

/**
 * Base trait for serialize/deserialize objects.
 * @tparam O type of plain object representation
 * @tparam B type of binary binary representation
 */
trait Codec[O, B] {

  def encode(obj: O): B

  def decode(binary: B): O

}
