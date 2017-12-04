package fluence.crypto

import scala.annotation.tailrec
import scala.collection.Searching.{ Found, InsertionPoint, SearchResult }
import scala.math.Ordering

/**
 * Wrapper for indexedSeq that provide search functionality over encrypted data.
 *
 * Example usage:
 * {{{
 *    import fluence.crypto.CryptoSearching._
 *    implicit val decryptFn = ???
 *    val l = List(enc("a"), enc("b"), enc("c"), enc("d"), enc("e"))
 *    l.search("c")
 *    // == Found(2)
 * }}}
 */
object CryptoSearching {

  class CryptoSearchImpl[A](coll: IndexedSeq[A]) {

    /**
     * Searches the specified indexedSeq for the search element using the binary search algorithm.
     * The sequence should be sorted with the same `Ordering` before calling, otherwise, the results are undefined.
     *
     * @param searchElem Search plaintext element.
     * @param decrypt     Decryption function for sequence elements.
     * @param ordering   The ordering to be used to compare elements.
     *
     * @return A `Found` value containing the index corresponding to the search element in the
     *         sequence. A `InsertionPoint` value containing the index where the element would be inserted if
     *         the search element is not found in the sequence.
     */
    final def binarySearch[B](searchElem: B)(implicit ordering: Ordering[B], decrypt: A ⇒ B): SearchResult = {
      binarySearchRec(searchElem, 0, coll.length, ordering, decrypt)
    }

    @tailrec
    private def binarySearchRec[B](elem: B, from: Int, to: Int, ordering: Ordering[B], decrypt: A ⇒ B): SearchResult = {

      if (from == to) return InsertionPoint(from)

      val idx = from + (to - from - 1) / 2
      math.signum(ordering.compare(elem, decrypt(coll(idx)))) match {
        case -1 ⇒ binarySearchRec(elem, from, idx, ordering, decrypt)
        case 1  ⇒ binarySearchRec(elem, idx + 1, to, ordering, decrypt)
        case _  ⇒ Found(idx)
      }
    }
  }

  implicit def search[A](indexedSeq: IndexedSeq[A]): CryptoSearchImpl[A] =
    new CryptoSearchImpl(indexedSeq)

  implicit def search[A](array: Array[A]): CryptoSearchImpl[A] =
    new CryptoSearchImpl(array)

}
