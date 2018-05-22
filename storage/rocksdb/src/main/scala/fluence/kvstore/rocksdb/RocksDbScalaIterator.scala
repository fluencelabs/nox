/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.kvstore.rocksdb

import org.rocksdb.RocksIterator

/**
 * Adopt [[org.rocksdb.RocksIterator]] to scala [[Iterator]] api.
 * If [[RocksDbScalaIterator#hasNext()]] return false, this iterator will be closed [[RocksIterator]] automatically.
 * '''Note! Before using this iterator should be invoked method [[this#start()]]'''
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
    require(rocksIterator != null, "rockIterator should not be null.")
    new RocksDbScalaIterator(rocksIterator)
  }
}
