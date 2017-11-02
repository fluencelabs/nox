package fluence.btree.binary

import scala.language.higherKinds

/**
 * Base trait for serialize/deserialize objects.
 * @tparam O type of plain object representation
 * @tparam B type of binary representation
 * @tparam F Encoding/decoding effect
 */
trait Codec[O, B, F[_]] {

  def encode(obj: O): F[B]

  def decode(binary: B): F[O]

}