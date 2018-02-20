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

package fluence.storage.rocksdb

import cats.syntax.functor._
import cats.{ Monad, ~> }
import fluence.codec.Codec
import monix.eval.Task
import monix.execution.atomic.AtomicLong

import scala.language.higherKinds

object IdSeqProvider {

  /**
   * Returns in memory id sequence provider for Longs.
   * Reads from local RocksDb a record with ''max key'' and create 'id sequence provider' started with ''max key''.
   * In case where there is no local RocksDb instance, makes 'id sequence provider' started with zero.
   *
   * @param rocksDB            Local RocksDb instance for scanning
   * @param defaultStartValue Start id value for new database (when specified RocksDn is empty)
   */
  def longSeqProvider[F[_] : Monad](
    rocksDB: RocksDbStore,
    defaultStartValue: Long = 0L
  )(implicit codec: Codec[Task, Array[Byte], Long], runTask: Task ~> F): F[() ⇒ Long] = {

    val idGenerator: Task[AtomicLong] =
      rocksDB
        .getMaxKey.attempt
        .flatMap {
          case Left(ex) ⇒
            Task(defaultStartValue)
          case Right(keyAsBytes) ⇒
            codec.encode(keyAsBytes)
        }.map {
          AtomicLong(_)
        }.memoizeOnSuccess // important to save this Atomic into Task as result

    runTask(idGenerator).map(provider ⇒ { () ⇒ provider.incrementAndGet() })
  }

}
