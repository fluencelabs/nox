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

package fluence.kad.core

import java.util.concurrent.TimeUnit

import cats.Monad
import cats.data.StateT
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.applicative._
import cats.effect.{Clock, LiftIO}
import fluence.kad.protocol.{KademliaRpc, Key, Node}

import scala.concurrent.duration.Duration

import scala.language.higherKinds

/**
 * Write ops are stateful
 *
 * @tparam F Effect
 * @tparam C Node contacts
 */
trait BucketsState[F[_], C] {

  /**
   * Returns current bucket state
   *
   * @param bucketId Bucket id, 0 to [[Key.BitLength]]
   */
  def read(bucketId: Int): F[Bucket[C]]

  /**
   * Returns current bucket state
   *
   * @param distanceKey Distance to get leading zeros from
   */
  def read(distanceKey: Key): F[Bucket[C]] =
    read(distanceKey.zerosPrefixLen)

  /**
   * Runs a mutation on bucket, blocks the bucket from writes until mutation is complete
   *
   * @param bucketId Bucket ID
   * @param mod      Mutation
   * @tparam T Return value
   */
  protected def run[T](bucketId: Int, mod: StateT[F, Bucket[C], T]): F[T]

  /**
   * Performs bucket update if necessary, blocking the bucket
   *
   * @param bucketId      Bucket ID
   * @param node          Fresh node
   * @param rpc           RPC caller for Kademlia functions
   * @param pingExpiresIn Duration for the ping to be considered relevant
   * @return True if node is updated in a bucket, false otherwise
   */
  def update(bucketId: Int, node: Node[C], rpc: C ⇒ KademliaRpc[C], pingExpiresIn: Duration)(
    implicit liftIO: LiftIO[F],
    F: Monad[F],
    clock: Clock[F]
  ): F[Boolean] =
    for {
      time ← clock.realTime(TimeUnit.MILLISECONDS)
      bucket ← read(bucketId)
      result ← if (bucket.shouldUpdate(node, pingExpiresIn, time))
        run(bucketId, Bucket.update(node, rpc, pingExpiresIn))
      else
        false.pure[F]
    } yield result

  /**
   * Removes a node from the bucket by node's key
   *
   * @param bucketId Bucket ID
   * @param key Key to remove
   * @param F Monad
   * @return Optional removed node
   */
  def remove(bucketId: Int, key: Key)(implicit F: Monad[F]): F[Option[Node[C]]] =
    run(bucketId, Bucket.remove(key))
}
