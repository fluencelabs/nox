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

package fluence.node.workers

import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.apply._
import cats.Monad
import cats.data.EitherT
import cats.effect.{Concurrent, Resource}
import cats.effect.concurrent.MVar
import fluence.effects.kvstore.{KVStore, KVStoreError}

import scala.collection.immutable.SortedSet
import scala.language.higherKinds
import scala.util.control.NoStackTrace

/**
 * Dispatcher for Worker Ports (local Tendermint p2p ports).
 *
 * @param available Set of available ports
 * @param store Persistent store of appId -> port
 * @param cache Ports mapping cache
 */
class WorkersPorts[F[_]: Monad] private (
  private val available: MVar[F, SortedSet[Short]],
  private val store: KVStore[F, Long, Short],
  private val cache: MVar[F, Map[Long, Short]]
) {

  /**
   * Get a port for an app, if it was allocated.
   *
   * @param appId Application ID
   * @return Some port, if it was allocated
   */
  def get(appId: Long): F[Option[Short]] =
    cache.read.map(_.get(appId))

  /**
   * Mapping of all allocated ports.
   *
   * @return appId -> port mapping
   */
  def getMapping: F[Map[Long, Short]] =
    cache.read

  /**
   * Get an allocated port for the app, or allocate a new one.
   *
   * @param appId Application ID
   * @return Left in case of error, or the port
   */
  def allocate(appId: Long): EitherT[F, WorkersPorts.Error, Short] =
    EitherT(
      // Read the cache, block on it
      cache.take.flatMap(
        m ⇒
          // Should we allocate a new port or is it already allocated
          m.get(appId) match {
            case Some(v) ⇒
              // Allocated -- release the cache by putting it back, and return the port
              cache.put(m).map(_ ⇒ Right(v))
            case None ⇒
              // Not allocated -- get available ports, block on them
              available.take.flatMap {
                // No more ports available!
                case av if av.isEmpty ⇒
                  // Release cache and available ports by putting the values back, intact
                  (available.put(av) *> cache.put(m))
                  // And return error
                    .map(_ ⇒ Left(WorkersPorts.Exhausted))
                case av ⇒
                  // Get the new port
                  val port = av.head
                  // Try to put the port to the persistent storage
                  store
                    .put(appId, port)
                    .leftMap(WorkersPorts.StoreError)
                    .value
                    .flatMap[Either[WorkersPorts.Error, Short]] {
                      case Left(err) ⇒
                        // Storage error: release available ports and cache
                        (available.put(av) *> cache.put(m))
                          .map(_ ⇒ Left(err))
                      case Right(_) ⇒
                        // Port stored: update availables and cache
                        (available.put(av.tail) *> cache.put(m + (appId -> port)))
                          .map(_ ⇒ Right(port))
                    }
              }
        }
      )
    )

  /**
   * Releases a port for an app, if it was previously allocated.
   *
   * @param appId Application ID
   * @return Some freed port, if it was allocated
   */
  def free(appId: Long): EitherT[F, KVStoreError, Option[Short]] =
    EitherT.right(get(appId)).flatMap {
      case Some(port) ⇒
        store
          .remove(appId)
          .flatMapF(
            _ ⇒
              for {
                // Remove from cache
                mapping ← cache.take
                _ ← cache.put(mapping - appId)

                // Add to available ports
                ports ← available.take
                _ ← available.put(ports + port)
              } yield Right(Some(port))
          )
      case None ⇒
        EitherT.rightT(None)
    }

}

object WorkersPorts {

  sealed trait Error extends Throwable with NoStackTrace

  /** Error on KVStore layer */
  case class StoreError(cause: KVStoreError) extends Error

  /** No more ports available for allocation */
  case object Exhausted extends Error

  /**
   * Make a new WorkersPorts instance.
   *
   * @param minPort Left bound of the range, included
   * @param maxPort Right bound of the range, included
   * @param store Persistent storage for the ports
   * @tparam F Concurrent for MVars
   * @return Prepared WorkersPorts instance
   */
  def make[F[_]: Concurrent](
    minPort: Short,
    maxPort: Short,
    store: KVStore[F, Long, Short]
  ): Resource[F, WorkersPorts[F]] =
    Resource.liftF(
      for {
        data ← store.stream.compile.toList.map(_.toMap)
        mapping ← MVar.of(data)
        available ← MVar.of(
          SortedSet.empty[Short] ++
            Range(minPort, maxPort + 1).to[Set].map(_.toShort) --
            data.values
        )
      } yield new WorkersPorts[F](available, store, mapping)
    )
}
