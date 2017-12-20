package fluence.btree.client.network

import fluence.btree.client.merkle.MerklePath
import fluence.btree.client.{ Bytes, Key, Value }

/**
 * Parent type for any response from server to client.
 */
sealed trait BTreeServerResponse

/**
 * Response from BTree server with details required for getting next child node index.
 *
 * @param keys              Keys of current branch
 * @param childsChecksums  All children checksums of current branch
 */
case class NextChildSearchResponse(
    keys: Array[Key],
    childsChecksums: Array[Bytes]
) extends BTreeServerResponse

/**
 * Response from server with current leaf details.
 *
 * @param keys    Keys of current leaf
 * @param values  Values of current leaf
 */
case class LeafResponse(
    keys: Array[Key],
    values: Array[Value]
) extends BTreeServerResponse

/** Parent type for any server response that mutates BTRee. */
sealed trait VerifyChanges extends BTreeServerResponse

/**
 * Response from server with new merkle root after inserting key/value.
 * This request require the client to confirm the new state before persisting.
 * This request tells client that rebalancing of tree isn't required and server just inserts into required leaf.
 *
 * @param merkleRoot Recalculated merkle root after putting new key/value to tree
 */
case class VerifySimplePutResponse(merkleRoot: Bytes) extends VerifyChanges

/**
 * Response from server to client after inserting key/value.
 * This request require the client to confirm the new state before persisting.
 * This request tells client that rebalancing of tree is required and server did rebalancing tree after insertion
 * into required leaf.
 *
 * @param merklePath New merkle path after putting new key-value to tree, that required client verification.
 */
// todo, actually MerklePath in current format isn't enough for verifying and format will be changed in future
case class VerifyPutWithRebalancingResponse(merklePath: MerklePath) extends VerifyChanges

/** Server response with accepting client confirmation. It means that server applied changes to BTree */
object ConfirmAccepted extends BTreeServerResponse
