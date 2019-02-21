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

package fluence.effects.kvstore
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.data.EitherT
import cats.effect.{Concurrent, Resource}
import cats.effect.concurrent.MVar

import scala.language.higherKinds

object MVarKVStore {

  /**
   * KVStore for tests usage, stores everything in a map inside MVar, all writes are blocking.
   *
   * @tparam F Concurrent for MVar
   * @tparam K Keys type
   * @tparam V Values type
   */
  def make[F[_]: Concurrent, K, V](): Resource[F, KVStore[F, K, V]] =
    Resource
      .liftF(
        MVar.of(Map.empty[K, V])
      )
      .map { data ⇒
        new KVStore[F, K, V] {
          override def get(key: K): EitherT[F, KVReadError, Option[V]] =
            EitherT.right(data.read.map(_.get(key)))

          override def put(key: K, value: V): EitherT[F, KVWriteError, Unit] =
            EitherT.right(for {
              d ← data.take
              _ ← data.put(d + (key -> value))
            } yield ())

          override def remove(key: K): EitherT[F, KVWriteError, Unit] =
            EitherT.right(for {
              d ← data.take
              _ ← data.put(d - key)
            } yield ())

          override def stream: fs2.Stream[F, (K, V)] =
            fs2.Stream
              .eval(data.read)
              .map(_.toIterator)
              .flatMap(fs2.Stream.fromIterator[F, (K, V)])
        }
      }
}
