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

package fluence.kvstore

import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import cats.syntax.flatMap._
import cats.{~>, Monad}
import fluence.kvstore.KVStore.{GetOp, PutOp, RemoveOp, TraverseOp}

import scala.collection.mutable
import scala.language.higherKinds
import scala.util.Try

class TestKVStore[K, V](
  protected val data: mutable.Map[K, V] = mutable.Map.empty[K, V]
) extends KVStore with ReadWriteKVStore[K, V] {

  override def get(key: K): GetOp[V] =
    new GetOp[V] {

      override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Option[V]] =
        EitherT.fromEither(
          Try(data.get(key)).toEither.left.map(err ⇒ StoreError.forGet(key, Some(err)))
        )

    }

  override def traverse: TraverseOp[K, V] = new TraverseOp[K, V] {

    override def run[FS[_]: Monad: LiftIO](
      implicit liftIterator: Iterator ~> FS
    ): FS[(K, V)] =
      IO(liftIterator(data.iterator)).to[FS].flatten

    override def runUnsafe: Iterator[(K, V)] =
      data.iterator

  }

  override def put(key: K, value: V): PutOp = new PutOp {

    override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Unit] =
      // format: off
      EitherT.fromEither {
        Try(data.put(key, value))
          .toEither
          .left.map(err ⇒ StoreError.forPut(key, value, Some(err)))
          .right.map(_ ⇒ ())
      }
    // format: on

  }

  override def remove(key: K): RemoveOp = new RemoveOp {

    override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Unit] =
      // format: off
      EitherT.fromEither {
        Try(data.remove(key))
          .toEither
          .left.map(err ⇒ StoreError.forRemove(key, Some(err)))
          .right.map(_ ⇒ ())
      }
    // format: on

  }

}
