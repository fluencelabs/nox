package fluence.btree.server

import cats.Show
import cats.syntax.show._

/**
 * ''Show'' type class implementations for [[MerkleBTree]] entities.
 */
object MerkleBTreeShow {

  implicit val showBytes: Show[Array[Byte]] =
    (b: Array[Byte]) ⇒ new String(b)

  implicit def showBytes(implicit sb: Show[Array[Byte]]): Show[Array[Array[Byte]]] =
    Show.show((array: Array[Array[Byte]]) ⇒ array.map(sb.show).mkString("[", ",", "]"))

  implicit def showLongArray: Show[Array[Long]] =
    Show.show((array: Array[Long]) ⇒ array.mkString("[", ",", "]"))

  implicit def showBranch(implicit sb: Show[Array[Array[Byte]]], sl: Show[Array[Long]]): Show[Branch] = {
    Show.show((t: Branch) ⇒ s"""Branch(${sb.show(t.keys)}, ${sl.show(t.children)}, ${t.size}, ${t.checksum.show})""")
  }

  implicit def showLeaf(implicit sb: Show[Array[Array[Byte]]], sl: Show[Array[Long]]): Show[Leaf] = {
    Show.show((l: Leaf) ⇒ s"""Leaf(${sb.show(l.keys)}, ${sb.show(l.values)}, ${l.size}, ${l.checksum.show})""")
  }

  implicit def showNode(implicit st: Show[Branch], sl: Show[Leaf]): Show[Node] = {
    case t: Branch ⇒ st.show(t)
    case l: Leaf   ⇒ sl.show(l)
  }

}
