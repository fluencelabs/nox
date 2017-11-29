package fluence.btree.client

import cats.syntax.show._
import fluence.btree.client.merkle.MerklePath

/**
 * Verifiable result of any ''tree'' operation.
 */
sealed trait VerifiableResult

/**
 * Results of Read operation (get).
 *
 * @param key    Searched key
 * @param value  Optional inserted or deleted value for put and delete operations
 * @param proof  Merkle proof for verifying current tree state and returned results
 */
case class ReadResults (
    key: Key,
    value: Option[Value],
    proof: MerklePath
) extends VerifiableResult {
  override def toString: String =
    s"ReadResults(key=${key.show}, value=${value.map(_.show)}, proof=$proof"
}

/**
 * Results of any write operation (put or delete).
 *
 * @param key             Searched key
 * @param value           Inserted value for put operation, deleted value for delete operation
 * @param oldStateProof  Merkle proof for verifying state before updating
 * @param newStateProof  Merkle proof for recalculating merkle root for state after updating
 */
case class WriteResults(
    key: Key,
    value: Value,
    oldStateProof: MerklePath,
    newStateProof: MerklePath,
) extends VerifiableResult {
  override def toString: String =
    s"WriteResults(key=${key.show}, value=${value.show}, oldStateProof=$oldStateProof, newStateProof=$newStateProof"
}
