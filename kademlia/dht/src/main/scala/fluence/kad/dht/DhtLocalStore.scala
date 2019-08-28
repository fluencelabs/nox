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

package fluence.kad.dht

import java.util.concurrent.TimeUnit

import cats.{Applicative, Monad}
import cats.data.EitherT
import cats.effect.concurrent.Ref
import cats.effect.{Clock, Concurrent, Fiber, Resource, Timer}
import cats.kernel.Semigroup
import cats.syntax.semigroup._
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.eq._
import fluence.codec.PureCodec
import fluence.crypto.Crypto
import fluence.effects.kvstore.{KVStore, ValueCodecError}
import fluence.kad.Kademlia
import fluence.kad.protocol.{Key, Node}
import fluence.log.Log
import scodec.bits.ByteVector

import scala.language.{higherKinds, postfixOps}
import scala.concurrent.duration._

/**
 * Local implementation for DhtRpc: store values in the local store, schedule refreshing.
 *
 * @param store Data storage
 * @param metadata Metadata storage
 * @param hasher Values hasher
 * @param scheduleRefresh Callback to schedule refresh of an updated (touched) value
 * @tparam F Effect
 * @tparam V Value; on update, new value is combined with the old one with [[Semigroup.combine]]
 */
class DhtLocalStore[F[_]: Monad: Clock, V: Semigroup](
  store: KVStore[F, Key, Array[Byte]],
  metadata: KVStore[F, Key, DhtValueMetadata],
  hasher: Crypto.Hasher[Array[Byte], ByteVector],
  scheduleRefresh: DhtLocalStore[F, V] ⇒ (Key, DhtValueMetadata) ⇒ F[Unit]
)(implicit codec: PureCodec[V, Array[Byte]])
    extends DhtRpc[F, V] {

  private val timestampT =
    EitherT.right[DhtError](Clock[F].realTime(TimeUnit.SECONDS))

  private val schedule = scheduleRefresh(this)

  /**
   * Retrieve the value from node's local storage
   *
   */
  override def retrieve(key: Key)(implicit log: Log[F]): EitherT[F, DhtError, V] =
    store
      .transformValues[V]
      .get(key)
      .leftMap(DhtLocalStoreError(_))
      .subflatMap(
        _.fold[Either[DhtError, V]](Left(DhtValueNotFound(key)))(Right(_))
      )

  /**
   * Kindly ask node to store the value in its local store.
   * Note that remote node may consider not to store the value or to modify it (e.g. combine with a semigroup).
   * You may need to check the value's consistency with a consequent [[retrieve]] call.
   *
   */
  override def store(key: Key, value: V)(implicit log: Log[F]): EitherT[F, DhtError, Unit] =
    for {
      // Ignore all errors while getting old value
      oldValueOpt ← EitherT.right[DhtError](
        store
          .transformValues[V]
          .get(key)
          .value
          .map(_.toOption.flatten)
      )

      // If the value for the key was previously stored, combine it with the new one
      newValue = oldValueOpt.fold(value)(_ |+| value)

      newBytes ← codec.direct(newValue).leftMap(e ⇒ DhtLocalStoreError(ValueCodecError(e)))

      newHash ← hasher(newBytes).leftMap(DhtCryptError)

      _ ← touch(key, newHash)

      _ ← store.put(key, newBytes).leftMap[DhtError](DhtLocalStoreError)
    } yield ()

  /**
   * Retrieve hash of the value, if it is stored
   *
   */
  override def retrieveHash(key: Key)(implicit log: Log[F]): EitherT[F, DhtError, ByteVector] =
    metadata
      .get(key)
      .leftMap(DhtLocalStoreError)
      .subflatMap(
        _.fold[Either[DhtError, ByteVector]](Left(DhtValueNotFound(key)))(m ⇒ Right(m.hash))
      )

  /**
   * Updates metadata associated with the key, schedules the next refreshing
   *
   * @param key Updated key
   * @param hash Updated value's hash
   * @param log Log
   */
  private[dht] def touch(key: Key, hash: ByteVector)(implicit log: Log[F]): EitherT[F, DhtError, Unit] =
    for {
      timestamp ← timestampT
      dhtMetadata = DhtValueMetadata(timestamp, hash)
      _ ← metadata.put(key, dhtMetadata).leftMap(DhtLocalStoreError(_))
      _ ← EitherT.right(schedule(key, dhtMetadata))
    } yield ()

  private def remove(key: Key)(implicit log: Log[F]): EitherT[F, DhtError, Unit] =
    (metadata.remove(key) >> store.remove(key)).leftMap[DhtError](DhtLocalStoreError)
}

object DhtLocalStore {

  /**
   * Republishes the value to Kademlia neighborhood of its Key
   *
   * @param local Local store that stores the value
   * @param key Key
   * @param kad Kademlia, used to determine the neighborhood
   * @param rpc Access to the DHT RPC for other Kademlia nodes
   * @param replication Desired replication level
   * @param maxCalls Maximum number of RPC STORE calls
   * @tparam F Effect
   * @tparam V Value
   * @tparam C Contact
   */
  private def refresh[F[_]: Monad, V, C](
    local: DhtLocalStore[F, V],
    key: Key,
    kad: Kademlia[F, C],
    rpc: C ⇒ DhtRpc[F, V],
    replication: Int,
    maxCalls: Int
  )(implicit log: Log[F]): EitherT[F, DhtError, Seq[Node[C]]] =
    for {
      // Fetch value's hash to avoid passing unchanged data
      hash ← local.retrieveHash(key)
      value ← local.retrieve(key)

      storedOn ← EitherT
        .right(
          // callIterative does its best to ensure that replication level is reached
          kad.callIterative[DhtError, Unit](
            key,
            n ⇒
              // This node is still in key's neighborhood
              if (n.key === kad.nodeKey) local.touch(key, hash)
              // Check whether remote node already stores the same value
              else
                rpc(n.contact).retrieveHash(key).flatMap {
                  case h if h == hash ⇒
                    // No need to do anything -- value is already stored
                    EitherT.rightT(())

                  case _ ⇒
                    // Store the value
                    rpc(n.contact).store(key, value)
                },
            replication,
            maxCalls,
            isIdempotentFn = false
          )
        )
        .map(_.map(_._1))

      // If value is no longer stored on this node, delete it
      _ ← if (!storedOn.exists(_.key === kad.nodeKey))
        local.remove(key)
      else
        EitherT.rightT[F, DhtError](())

    } yield storedOn

  /**
   * Makes a new DhtLocalStore instance, running all the expected background jobs (namely, refreshing).
   *
   * @param store Local values storage
   * @param metadata Local metadata storage
   * @param hasher Values hasher
   * @tparam F Effect
   * @tparam V Value
   */
  def make[F[_]: Monad: Timer: Concurrent, V: Semigroup, C](
    store: KVStore[F, Key, Array[Byte]],
    metadata: KVStore[F, Key, DhtValueMetadata],
    hasher: Crypto.Hasher[Array[Byte], ByteVector],
    kademlia: Kademlia[F, C],
    rpc: C ⇒ DhtRpc[F, V],
    conf: Dht.Conf
  )(implicit codec: PureCodec[V, Array[Byte]], log: Log[F]): Resource[F, DhtLocalStore[F, V]] =
    Resource
      .liftF(Ref.of[F, Map[Key, Fiber[F, Unit]]](Map.empty))
      .flatMap { scheduled ⇒
        def scheduleRefresh(local: DhtLocalStore[F, V])(key: Key, meta: DhtValueMetadata): F[Unit] =
          // Remove and cancel an old fiber, if it exists
          scheduled
            .modify(m ⇒ (m - key, m.get(key)))
            .flatMap(_.fold(Applicative[F].unit)(_.cancel)) >>
            // Run new fiber
            Concurrent[F]
              .start(
                Log[F].scope("dht-refresh") { implicit log: Log[F] ⇒
                  Clock[F]
                    .realTime(TimeUnit.SECONDS)
                    .map(_ - meta.lastUpdated)
                    .map(conf.refreshPeriod.toSeconds - _)
                    .flatMap {
                      case sleepSecondsLeft if sleepSecondsLeft > 0 ⇒
                        Timer[F].sleep(sleepSecondsLeft.seconds)
                      case _ ⇒
                        Applicative[F].unit
                    } >>
                    refresh(local, key, kademlia, rpc, conf.replicationFactor, conf.maxStoreCalls).value.void
                }
              )
              // Store new fiber
              .flatMap(f ⇒ scheduled.update(_ + (key -> f)))

        Resource
          .pure(new DhtLocalStore[F, V](store, metadata, hasher, scheduleRefresh))
          // Schedule refresh for old records
          .flatTap(
            local ⇒
              Resource.make(
                Log[F].scope("dht-stream") { implicit log: Log[F] ⇒
                  Concurrent[F].start(
                    metadata.stream
                      .evalTap[F] {
                        case (k, md) ⇒ scheduleRefresh(local)(k, md)
                      }
                      .compile
                      .drain
                  )
                }
              )(_.cancel)
          )

      }
}
