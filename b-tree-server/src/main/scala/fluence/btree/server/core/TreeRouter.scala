package fluence.btree.server.core

import cats.Applicative

import scala.collection.Searching.SearchResult
import scala.math.Ordering

/**
 * Router helps to find key position in tree node keys.
 *
 * @tparam K The type of search key
 * @tparam R The type returned results
 * @tparam F The type of effect, box for returning value
 */
trait TreeRouter[K, R, F[_]] {

  /* todo. improve api to make possible prevent next attacks:

    1 Why should the client (it's router, right?) tell server the insertion point if leaf.keys didn't contain the key? Doesn't it leak some unnecessary information?
    2 Should the client check that leaf.keys is coming from the correct node? What if server supplies leaf.keys from some other node?
    3 What is going to happen with the client if server supplies leaf.keys shuffled? I guess conventional binary search is not going to work?
    4 What is going to happen if the server passes into router.indexOf method not the key that client asked it for, but a different key from this tree?

   1 use for get request different method, server should know insertion idx when key was not found in tree
   2 and 4
    client should check some 'merkle proof' for any round trip for current search request,
    any round trip client should have all information for getting merkle root and compare it
   3  client should checks search key before start answering or key should always be only on the client side (second is preferable)

  */

  def indexOf(key: K, keys: Array[K]): F[R]

}

/**
 * Local implementation of [[TreeRouter]] with simple binary searching based on standard comparing elements.
 *
 * @tparam K type of search key
 * @tparam F type of effect, box for returning value
 */
class LocalRouter[K : Ordering, F[_] : Applicative] extends TreeRouter[K, SearchResult, F] {
  import scala.collection.Searching._

  override def indexOf(key: K, keys: Array[K]): F[SearchResult] = {
    Applicative[F].pure(keys.search(key))
  }
}

object LocalRouter {
  def apply[K : Ordering, F[_] : Applicative]: LocalRouter[K, F] = {
    new LocalRouter[K, F]
  }
}
