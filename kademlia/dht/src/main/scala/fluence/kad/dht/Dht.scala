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
import fluence.kad.Kademlia
import fluence.kad.protocol.Key
import fluence.log.Log

import scala.language.higherKinds

trait Dht[F[_], V] {
  def retrieve(key: Key)(implicit log: Log[F]): EitherT[F, DhtError, V]

  def store(key: Key, value: V)(implicit log: Log[F]): EitherT[F, DhtError, Unit]
}

object Dht {
  case class Conf(
    retrieveResults: Int = 1,
    maxRetrieveCalls: Int = 8,
    replicationFactor: Int = 4,
    maxStoreCalls: Int = 8
  )

  def apply[F[_]: Monad, V: Semigroup, C](
    kad: Kademlia[F, C],
    rpc: C ⇒ Dht[F, V],
    conf: Conf
  ): Dht[F, V] =
    new Impl(kad, rpc, conf)

  class Impl[F[_]: Monad, V: Semigroup, C](
    kad: Kademlia[F, C],
    rpc: C ⇒ Dht[F, V],
    conf: Conf
  ) extends Dht[F, V] {

    import conf._

    override def retrieve(key: Key)(implicit log: Log[F]): EitherT[F, DhtError, V] =
      EitherT(
        kad.callIterative(key, n ⇒ rpc(n.contact).retrieve(key), retrieveResults, maxRetrieveCalls).map {
          case sq if sq.isEmpty ⇒
            Left(DhtValueNotFound(key))
          case sq ⇒
            Right(sq.map(_._2).reduce(_ |+| _))
        }
      )

    override def store(key: Key, value: V)(implicit log: Log[F]): EitherT[F, DhtError, Unit] =
      EitherT(
        kad
          .callIterative(key,
                         n ⇒ rpc(n.contact).store(key, value),
                         replicationFactor,
                         maxStoreCalls,
                         isIdempotentFn = false)
          .flatMap {
            case sq if sq.isEmpty ⇒
              Applicative[F].pure(Left(DhtCannotStoreValue(key)))
            case sq ⇒
              (
                if (sq.length < replicationFactor)
                  log.warn(s"For key $key required replication factor $replicationFactor, reached just ${sq.length}")
                else
                  Applicative[F].unit
              ).as(Right(()))
          }
      )
  }
}
