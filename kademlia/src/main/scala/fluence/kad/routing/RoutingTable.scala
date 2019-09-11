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

package fluence.kad.routing

import java.nio.file.Path

import cats.effect.{Async, Clock, Concurrent, ContextShift, LiftIO, Resource, Timer}
import cats.{Defer, Monad, Parallel, Traverse}
import cats.instances.list._
import cats.syntax.flatMap._
import cats.syntax.applicative._
import cats.syntax.functor._
import cats.syntax.compose._
import fluence.codec.{CodecError, PureCodec}
import fluence.crypto.Crypto
import fluence.effects.kvstore.{KVStore, RocksDBStore}
import fluence.kad.conf.RoutingConf
import fluence.kad.contact.ContactAccess
import fluence.kad.protocol.{Key, Node}
import fluence.kad.state.RoutingState
import fluence.log.Log

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

/**
 * Wraps all the routing state and logic
 *
 * @param local Local routing, used in iterative routing
 * @param iterative Iterative routing
 * @param state Internal state, used for routing
 * @tparam F Effect
 * @tparam C Contact
 */
case class RoutingTable[F[_], C](
  local: LocalRouting[F, C],
  iterative: IterativeRouting[F, C],
  private[kad] val state: RoutingState[F, C]
) {

  def nodeKey: Key = local.nodeKey

}

object RoutingTable {

  /**
   * Wraps the logic of applying an extension while building the [[RoutingTable]], see [[apply]]
   *
   * @param modifyState Modify state, prior to building on it
   * @param modifyLocal Modify local routing logic, prior to building on it
   * @param modifyIterative Modify iterative routing logic, prior to building on it
   * @tparam F Effect
   * @tparam C Contact
   */
  sealed class Extension[F[_], C](
    private[routing] val modifyState: RoutingState[F, C] ⇒ F[RoutingState[F, C]],
    private[routing] val modifyLocal: LocalRouting[F, C] ⇒ F[LocalRouting[F, C]],
    private[routing] val modifyIterative: IterativeRouting[F, C] ⇒ F[IterativeRouting[F, C]]
  )

  /**
   * Extension: see [[RoutingState.bootstrapWithStore]]
   *
   * @param store Store to bootstrap from, and to save nodes to
   */
  def bootstrapWithStore[F[_]: Concurrent: Clock: Log, C](
    store: KVStore[F, Key, Node[C]]
  )(implicit ca: ContactAccess[F, C]): Extension[F, C] =
    new Extension[F, C](
      state ⇒ RoutingState.bootstrapWithStore(state, store),
      _.pure[F],
      _.pure[F]
    )

  /**
   * Extension: see [[RefreshingIterativeRouting]]
   *
   * @param refreshTimeout Frequency for each bucket's refresh, should be around an hour or more
   * @param refreshNeighbors How many neighbors to lookup on refresh
   * @param parallelism Refreshing parallelism, should be taken from KademliaConf
   */
  def refreshing[F[_]: Concurrent: Timer: Log, C](
    refreshTimeout: FiniteDuration,
    refreshNeighbors: Int,
    parallelism: Int
  ): Extension[F, C] =
    new Extension[F, C](
      _.pure[F],
      _.pure[F],
      iterative ⇒ RefreshingIterativeRouting(iterative, refreshTimeout, refreshNeighbors, parallelism)
    )

  /**
   * Builds a Refreshing extension, if it's enabled in [[RoutingConf]]
   *
   * @param conf Routing configuration, see [[RoutingConf.refreshing]]
   */
  def refreshingOpt[F[_]: Concurrent: Timer: Log, C](
    conf: RoutingConf
  ): Option[Extension[F, C]] =
    conf.refreshing
      .filter(_.period.isFinite())
      .fold[Option[Extension[F, C]]](None)(
        r ⇒
          Some(
            refreshing[F, C](
              r.period.asInstanceOf[FiniteDuration], // Checked with .filter
              r.neighbors,
              conf.parallelism
            )
          )
      )

  /**
   * Prepare Kademlia Cache Store extension, using Rocksdb as a backend, if it's enabled in [[RoutingConf]]
   *
   * @param conf Routing config
   * @param rootPath RocksDB storage root path
   * @param ca Contact access
   * @param writeNode Serialize Node to string
   * @param readNode Parse Node from string, checking all the signatures on the way
   * @tparam F Effect
   * @tparam C Contact
   * @return Extension resource
   */
  def rocksdbStoreExtResource[F[_]: LiftIO: ContextShift: Log: Concurrent: Clock, C](
    conf: RoutingConf,
    rootPath: Path
  )(
    implicit
    ca: ContactAccess[F, C],
    writeNode: PureCodec.Func[Node[C], String],
    readNode: Crypto.Func[String, Node[C]]
  ): Resource[F, Option[Extension[F, C]]] =
    conf.store
      .fold(Resource.pure[F, Option[Extension[F, C]]](None)) { cachePath ⇒
        val nodeCodec: PureCodec[String, Node[C]] =
          PureCodec.build(
            PureCodec.fromOtherFunc(readNode)(ee ⇒ CodecError("Cannot decode Node due to Crypto error", Some(ee))),
            writeNode
          )

        implicit val nodeBytesCodec: PureCodec[Array[Byte], Node[C]] =
          nodeCodec compose PureCodec
            .liftB[Array[Byte], String](bs ⇒ new String(bs), _.getBytes())

        RocksDBStore
          .make[F, Key, Node[C]](rootPath.resolve(cachePath).toAbsolutePath.toString)
          .map(bootstrapWithStore[F, C](_))
          .map(Some(_))
      }

  /**
   * Build an in-memory Kademlia state, apply extensions on it
   *
   * @param nodeKey Current node's key
   * @param siblingsSize Number of siblings to store in [[fluence.kad.state.Siblings]] state
   * @param maxBucketSize Number of nodes to store in each [[fluence.kad.state.Bucket]] state
   * @param extensions Extensions to apply
   * @return Ready-to-use RoutingTable, expected to be a singleton
   */
  def apply[F[_]: Async: Parallel: Clock: LiftIO, P[_], C](
    nodeKey: Key,
    siblingsSize: Int,
    maxBucketSize: Int,
    extensions: List[Extension[F, C]] = Nil
  )(implicit ca: ContactAccess[F, C]): F[RoutingTable[F, C]] =
    for {
      // Build a plain in-memory routing state
      st ← RoutingState.inMemory[F, P, C](nodeKey, siblingsSize, maxBucketSize)

      // Apply extensions to the state, use extended version then
      state ← Traverse[List].foldLeftM(extensions, st) {
        case (s, ext) ⇒ ext.modifyState(s)
      }

      // Extend local routing, using extended state
      loc = LocalRouting(state.nodeKey, state.siblings, state.bucket)
      local ← Traverse[List].foldLeftM(extensions, loc) {
        case (l, ext) ⇒ ext.modifyLocal(l)
      }

      // Extend iterative routing, using extended local routing and state
      it = IterativeRouting(local, state)
      iterative ← Traverse[List].foldLeftM(extensions, it) {
        case (i, ext) ⇒ ext.modifyIterative(i)
      }

      // Yield routing, aggregating all the extensions inside
    } yield RoutingTable(local, iterative, state)

}
