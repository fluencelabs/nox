package fluence.btree.client.network

import fluence.btree.client.{ Key, Value }
import scala.collection.Searching.SearchResult

/** Parent type for any requests from client to server. */
sealed trait BTreeClientRequest

/** Parent type for initial requests from client to server. */
sealed trait InitClientRequest extends BTreeClientRequest

/** Initial request for start getting value from server BTree. */
object InitGetRequest extends InitClientRequest

/** Initial request for start putting value from server BTree. */
object InitPutRequest extends InitClientRequest

/**
 * Request for resuming descent down the tree.
 *
 * @param nextChildIdx Next child node index, for making next step down the tree.
 */
case class ResumeSearchRequest(nextChildIdx: Int) extends BTreeClientRequest

/**
 * Request for putting new key and value into tree.
 *
 * @param key           Encrypted key
 * @param value         Encrypted value
 * @param searchResult Result of searching client key in server leaf keys. Contains an index
 *                       for putting specified key and value
 */
case class PutRequest(key: Key, value: Value, searchResult: SearchResult) extends BTreeClientRequest

/** Parent type for any final client request for any mutation operation into the tree. */
sealed trait FinishClientRequest extends BTreeClientRequest

/** Confirmation for last response from server BTree. The client accepted the server response */
object Confirm extends FinishClientRequest

/** Rejection for last response from server BTree. The client did not accept the server response */
object Reject extends FinishClientRequest
