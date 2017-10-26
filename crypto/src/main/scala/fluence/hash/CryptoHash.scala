package fluence.hash

/**
 * Base interface for hashing.
 * @tparam M type of message for hashing
 * @tparam H type of hashed message
 */
trait CryptoHash[M, H] {

  def hash(msg: M): H

  def hash(msg1: M, msgN: M*): H

}