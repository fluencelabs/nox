package fluence.node.storage.rocksdb

import org.rocksdb.RocksIterator


/**
 * Adopt [[org.rocksdb.RocksIterator]] to scala [[Iterator]] api.
 * If [[RocksDbScalaIterator#hasNext()]] return false, this iterator will be closed [[RocksIterator]] automatically.
 * '''Note! Before using this iterator should be eval method [[this#start()]]'''
 *
 * @param rocksIterator rocksDB iterator (cursor)
 */
class RocksDbScalaIterator(rocksIterator: RocksIterator) extends Iterator[(Array[Byte], Array[Byte])] {

  start()

  /**
   * Returns true if current cursor position is valid.
   * Close RocksIterator when [[rocksIterator#isValid()]] return false for current position of cursor.
   */
  override def hasNext: Boolean = {
    rocksIterator.isValid || closeIterator()
  }

  /**
   * Return current valid pair (K, V) and move cursor to next position.
   */
  override def next(): (Array[Byte], Array[Byte]) = {
    val result = rocksIterator.key() -> rocksIterator.value()
    rocksIterator.next()
    result
  }

  private def start() = {
    rocksIterator.seekToFirst()
  }

  private def closeIterator(): Boolean = {
    rocksIterator.close()
    false
  }
}


object RocksDbScalaIterator {
  def apply(rocksIterator: RocksIterator): RocksDbScalaIterator = {
    new RocksDbScalaIterator(rocksIterator)
  }
}
