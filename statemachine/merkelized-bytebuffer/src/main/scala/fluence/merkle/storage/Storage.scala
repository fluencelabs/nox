package fluence.merkle.storage

import java.util

trait Storage[Element] {
  def getElements(offset: Int, length: Int): Element
  def getDirtyChunks: util.BitSet
}
