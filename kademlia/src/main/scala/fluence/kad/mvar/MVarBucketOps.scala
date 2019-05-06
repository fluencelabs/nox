/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.kad.mvar

import cats.Traverse
import cats.data.StateT
import cats.effect.Async
import cats.syntax.functor._
import fluence.kad.core.{Bucket, BucketsState}
import fluence.kad.protocol.Key

import scala.language.higherKinds

object MVarBucketOps {

  /**
   * Implementation of Bucket.WriteOps, based on MVar and TrieMap -- available only on JVM
   *
   * @param maxBucketSize Max number of nodes in each bucket
   * @tparam C Node contacts
   */
  def apply[F[_]: Async, C](maxBucketSize: Int, numOfBuckets: Int = Key.BitLength): F[BucketsState[F, C]] = {
    import cats.instances.stream._

    val emptyBucket = Bucket[C](maxBucketSize)

    Traverse[Stream]
      .traverse(Stream.range(0, numOfBuckets))(
        i ⇒
          ReadableMVar
            .of[F, Bucket[C]](emptyBucket)
            .map(i → _)
      )
      .map(_.toMap)
      .map { state ⇒
        new BucketsState[F, C] {

          /**
           * Runs a mutation on bucket, blocks the bucket from writes until mutation is complete
           *
           * @param bucketId Bucket ID
           * @param mod      Mutation
           * @tparam T Return value
           */
          override protected def run[T](bucketId: Int, mod: StateT[F, Bucket[C], T]): F[T] =
            state(bucketId).apply(mod)

          /**
           * Returns current bucket state
           *
           * @param bucketId Bucket id, 0 to [[Key.BitLength]]
           */
          override def read(bucketId: Int): F[Bucket[C]] = state(bucketId).read
        }
      }
  }
}
