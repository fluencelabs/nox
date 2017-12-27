package fluence.btree.common

import cats.Show

/**
 * ''Show'' type class implementations for common btree entities.
 */
object BTreeCommonShow {

  implicit val showBytes: Show[Array[Byte]] =
    (b: Array[Byte]) ⇒ new String(b)

  implicit def showArrayOfBytes(implicit sb: Show[Array[Byte]]): Show[Array[Array[Byte]]] =
    Show.show((array: Array[Array[Byte]]) ⇒ array.map(sb.show).mkString("[", ",", "]"))

  implicit def showPutDetails(implicit sb: Show[Array[Byte]]): Show[PutDetails] = {
    Show.show((pd: PutDetails) ⇒ s"""PutDetails(${sb.show(pd.key)}, ${sb.show(pd.value)}, ${pd.searchResult})""")
  }

}
