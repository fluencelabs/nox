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

package fluence.kad

import cats.Monad
import cats.data.StateT
import monix.eval.{ MVar, Task }

import scala.collection.concurrent.TrieMap

/**
 * Implementation of Bucket.WriteOps, based on MVar and TrieMap -- available only on JVM
 *
 * @param maxBucketSize Max number of nodes in each bucket
 * @tparam C Node contacts
 */
class TrieMapMVarBucketOps[C](maxBucketSize: Int) extends Bucket.WriteOps[Task, C] {
  private val writeState = TrieMap.empty[Int, MVar[Bucket[C]]]
  private val readState = TrieMap.empty[Int, Bucket[C]]

  import RunOnMVar.runOnMVar

  override protected implicit def F: Monad[Task] = Monad[Task]

  override protected def run[T](bucketId: Int, mod: StateT[Task, Bucket[C], T]): Task[T] =
    runOnMVar(
      writeState.getOrElseUpdate(bucketId, MVar(read(bucketId))),
      mod,
      readState.update(bucketId, _: Bucket[C])
    )

  override def read(bucketId: Int): Bucket[C] =
    readState.getOrElseUpdate(bucketId, Bucket[C](maxBucketSize))
}
