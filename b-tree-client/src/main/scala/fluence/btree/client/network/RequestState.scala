package fluence.btree.client.network

import fluence.btree.client.{ Bytes, Value }
import fluence.btree.client.common.BytesOps
import fluence.btree.client.core.ClientState
import fluence.btree.client.merkle.MerklePath

/**
 * State of any request from client to server.
 */
sealed trait RequestState

/**
 * State for each 'Get' request to remote BTree. One ''GetState'' corresponds to one series of round trip requests
 *
 * @param key         The search plain text ''key''
 * @param merkleRoot  Copy of client merkle root at the beginning of the request
 * @param merklePath  Tree path traveled on the server
 * @param nextRequest Next request to server
 * @tparam K The type of plain text ''key''
 */
case class GetState[K] private (
    key: K,
    merkleRoot: Array[Byte],
    merklePath: MerklePath,
    nextRequest: BTreeClientRequest
) extends RequestState

object GetState {
  def apply[K](key: K, state: ClientState, nextRequest: BTreeClientRequest = InitGetRequest): GetState[K] =
    new GetState(key, BytesOps.copyOf(state.merkleRoot), MerklePath.empty, nextRequest)
}

/**
 * State for each 'Put' request to remote BTree. One ''PutState'' corresponds to one series of round trip requests
 *
 * @param key            The search plain text ''key''
 * @param value          Plain text ''value'' to be inserted to server BTree
 * @param merkleRoot     Copy of client merkle root at the beginning of the request
 * @param merklePath     Tree path traveled on the server
 * @param nextRequest    Next request to server
 * @param oldCipherValue An old value that will be rewritten or None if key for putting wasn't present in B Tree
 * @tparam K The type of plain text ''key''
 * @tparam V The type of plain text ''value''
 */
case class PutState[K, V] private (
    key: K,
    value: V,
    merkleRoot: Bytes,
    merklePath: MerklePath,
    nextRequest: BTreeClientRequest,
    oldCipherValue: Option[Value]
) extends RequestState

object PutState {
  def apply[K, V](
    key: K,
    value: V,
    state: ClientState,
    nextRequest: BTreeClientRequest = InitPutRequest,
    oldCipherValue: Option[Value] = None
  ): PutState[K, V] =
    new PutState(key, value, BytesOps.copyOf(state.merkleRoot), MerklePath.empty, nextRequest, oldCipherValue)
}
