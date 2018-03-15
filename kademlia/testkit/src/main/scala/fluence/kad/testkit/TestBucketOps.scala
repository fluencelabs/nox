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

package fluence.kad.testkit

import cats.Monad
import cats.data.StateT
import fluence.kad.Bucket
import monix.eval.Coeval

import scala.language.higherKinds

class TestBucketOps[C](maxBucketSize: Int) extends Bucket.WriteOps[Coeval, C] {
  self ⇒

  private val buckets = collection.mutable.Map.empty[Int, Bucket[C]]
  private val locks = collection.mutable.Map.empty[Int, Boolean].withDefaultValue(false)

  override protected implicit def F: Monad[Coeval] = Monad[Coeval]

  override protected def run[T](bucketId: Int, mod: StateT[Coeval, Bucket[C], T]) = {
    require(!locks(bucketId), s"Bucket $bucketId must be not locked")
    locks(bucketId) = true
    //println(s"Bucket $bucketId locked")

    mod.run(read(bucketId)).map {
      case (b, v) ⇒
        buckets(bucketId) = b
        v
    }.doOnFinish{ _ ⇒
      //println(s"Bucket $bucketId unlocked")
      Coeval.now(locks.update(bucketId, false))
    }
  }

  override def read(bucketId: Int): Bucket[C] =
    buckets.getOrElseUpdate(bucketId, Bucket(maxBucketSize))

}
