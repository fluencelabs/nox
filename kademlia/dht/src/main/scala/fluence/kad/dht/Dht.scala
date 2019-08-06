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

import cats.{Applicative, Monad}
import cats.data.EitherT
import cats.kernel.Semigroup
import cats.syntax.semigroup._
import cats.syntax.functor._
import cats.syntax.flatMap._
import fluence.effects.kvstore.{IOExceptionError, KVReadError, KVStore, KVWriteError, UnsupportedOperationError}
import fluence.kad.Kademlia
import fluence.kad.dht.Dht.Conf
import fluence.kad.protocol.Key
import fluence.log.Log

import scala.concurrent.duration._
import scala.language.higherKinds

/**
 * Kademlia DHT access point, provides a [[KVStore]] interface over a distributed hash table.
 * As Kademlia addresses are always [[Key]], you need to provide some mapping from your custom keys, if any,
 * into [[Key]]. There's probably no way back, as [[Key]] is usually made with hashing function.
 * Authentication should be done inside `rpc` function. Values republishing currently is also out of scope of this class.
 *
 * TODO provide republishing
 *
 * @param kad Kademlia instance to rely on
 * @param rpc Access to remote RPC endpoints
 * @param conf Default configuration for [[KVStore]] functions
 * @tparam F Effect
 * @tparam V Value; [[Semigroup]] is used to merge several returned values
 * @tparam C Contact
 */
class Dht[F[_]: Monad, V: Semigroup, C](
  kad: Kademlia[F, C],
  rpc: C ⇒ DhtRpc[F, V],
  conf: Conf = Dht.Conf()
) extends KVStore[F, Key, V] {

  override def get(key: Key)(implicit log: Log[F]): EitherT[F, KVReadError, Option[V]] =
    retrieve(key, conf.retrieveResults, conf.maxRetrieveCalls)
      .map[Option[V]](Some(_))
      .leftFlatMap[Option[V], KVReadError] {
        case _: DhtValueNotFound ⇒ EitherT.rightT(None)
        case err ⇒ EitherT.leftT(IOExceptionError(s"Unable to retrieve value from DHT, key = $key", err))
      }

  override def put(key: Key, value: V)(implicit log: Log[F]): EitherT[F, KVWriteError, Unit] =
    store(key, value, conf.replicationFactor, conf.maxStoreCalls)
      .leftMap(err ⇒ IOExceptionError(s"Unable to store value into DHT, key = $key", err))

  /**
   * Unsupported: there's no guarantees to remove a value from DHT
   *
   * @param key Key
   */
  override def remove(key: Key)(implicit log: Log[F]): EitherT[F, KVWriteError, Unit] =
    EitherT.leftT(UnsupportedOperationError(s"Cannot remove from Kademlia DHT, key = $key"))

  /**
   * Unsupported: there's no straightforward way to iterate over all DHT values
   *
   */
  override def stream(implicit log: Log[F]): fs2.Stream[F, (Key, V)] =
    fs2.Stream.eval_(log.error("Kademlia DHT doesn't support KVStore's streaming"))

  /**
   * Retrieve a result from DHT: collect up to `results` values, merge them with [[Semigroup]], doing maximum of `maxCalls`
   * RPC calls. In case no value is found across the network, [[DhtValueNotFound]] is returned.
   *
   * @param key Key to retrieve
   * @param results Number of results to collect and merge
   * @param maxCalls Maximum number of network calls
   */
  def retrieve(key: Key, results: Int, maxCalls: Int)(implicit log: Log[F]): EitherT[F, DhtError, V] =
    EitherT(
      kad.callIterative(key, n ⇒ rpc(n.contact).retrieve(key), results, maxCalls).map {
        case sq if sq.isEmpty ⇒
          Left(DhtValueNotFound(key))
        case sq ⇒
          Right(sq.map(_._2).reduce(_ |+| _))
      }
    )

  /**
   * Stores the value in the DHT, trying to reach the required `replication` factor, making no more then `maxCalls` RPC calls.
   *
   * @param key Key
   * @param value Value
   * @param replication Replication factor: how many remote nodes should store the value
   * @param maxCalls The upper bound for an overall number of calls made, see [[Kademlia.callIterative]] for details
   */
  def store(key: Key, value: V, replication: Int, maxCalls: Int)(implicit log: Log[F]): EitherT[F, DhtError, Unit] =
    EitherT(
      kad
        .callIterative(key, n ⇒ rpc(n.contact).store(key, value), replication, maxCalls, isIdempotentFn = false)
        .flatMap {
          case sq if sq.isEmpty ⇒
            Applicative[F].pure(Left(DhtCannotStoreValue(key)))
          case sq ⇒
            (
              if (sq.length < replication)
                Log[F].warn(s"For key $key required replication factor $replication, reached just ${sq.length}")
              else
                Applicative[F].unit
            ).as(Right(()))
        }
    )
}

object Dht {

  /**
   * Default configuration for [[Dht]] class.
   *
   * @param retrieveResults How many results try to retrieve before halting the retrieve operation
   * @param maxRetrieveCalls How many RPC calls are allowed during retrieve operation
   * @param replicationFactor How many copies of the value to store
   * @param maxStoreCalls How many RPC calls are allowed during store operation
   */
  case class Conf(
    retrieveResults: Int = 2,
    maxRetrieveCalls: Int = 8,
    replicationFactor: Int = 4,
    maxStoreCalls: Int = 16,
    refreshPeriod: FiniteDuration = 1.hour
  )

}
