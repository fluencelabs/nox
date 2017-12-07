package fluence.btree.client.common

import java.util

import fluence.btree.client.Bytes

/** Frequently used operations with bytes. */
object BytesOps {

  /** Returns copy of bytes. */
  def copyOf(bytes: Bytes): Bytes =
    util.Arrays.copyOf(bytes, bytes.length)

  /** Returns shallow copy of specified array. Note that, this method doesn't copy array elements! */
  def copyOf(array: Array[Bytes]): Array[Bytes] =
    util.Arrays.copyOf(array, array.length)

  /**
   * Returns updated shallow copy of array with the updated element for ''insIdx'' index.
   * We choose variant with array copying for prevent changing input parameters.
   * Work with mutable structures is more error-prone. It may be changed in the future by performance reason.
   */
  def rewriteValue(array: Array[Bytes], insBytes: Bytes, insIdx: Int): Array[Bytes] = {
    val newArray = copyOf(array)
    newArray(insIdx) = insBytes
    newArray
  }

}
