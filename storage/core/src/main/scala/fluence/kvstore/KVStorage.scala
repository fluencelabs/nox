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

import cats.{Applicative, MonadError}
import fluence.kvstore.ops.Get.KVStoreGet
import fluence.kvstore.ops.Put.KVStorePut
import fluence.kvstore.ops.Remove.KVStoreRemove
import fluence.kvstore.ops.Traverse.KVStoreTraverse

import scala.language.higherKinds

/**
 * Top type for any key value storage.
 */
trait KVStorage

/**
 * Key-value storage api for reading values.
 *
 * @tparam K The type of keys
 * @tparam V The type of stored values
 * @tparam E The type of storage error
 */
trait KVStoreRead[K, V, E <: StoreError] extends KVStoreGet[K, V, E] with KVStoreTraverse[K, V, E]

/**
 * Key-value storage api for writing values.
 *
 * @tparam K The type of keys
 * @tparam V The type of stored values
 * @tparam E The type of storage error
 */
trait KVStoreWrite[K, V, E <: StoreError] extends KVStorePut[K, V, E] with KVStoreRemove[K, E]

/**
 * Key-value storage api for reading and writing.
 *
 * @tparam K The type of keys
 * @tparam V The type of stored values
 * @tparam E The type of storage error
 */
trait ReadWriteKVStore[K, V, E <: StoreError] extends KVStoreRead[K, V, E] with KVStoreWrite[K, V, E]

/**
 * Key-value storage api for getting storage snapshot.
 *
 * @tparam S The type of returned storage snapshot.
 */
trait Snapshot[S <: KVStorage] {

  def createSnapshot[F[_]: Applicative](): F[S]

}

object KVStorage {

  // this MonadError is needed for travers and runF operations
  implicit def storeMonadError[F[_]](implicit ME: MonadError[F, Throwable]): MonadError[F, StoreError] =
    new MonadError[F, StoreError] {
      override def flatMap[A, B](fa: F[A])(f: A ⇒ F[B]): F[B] = ME.flatMap(fa)(f)
      override def tailRecM[A, B](a: A)(f: A ⇒ F[Either[A, B]]): F[B] = ME.tailRecM(a)(f)
      override def raiseError[A](e: StoreError): F[A] = ME.raiseError(e)
      override def handleErrorWith[A](fa: F[A])(f: StoreError ⇒ F[A]): F[A] = ME.handleErrorWith(fa) {
        case cf: StoreError ⇒ f(cf)
        case t ⇒ ME.raiseError(t)
      }
      override def pure[A](x: A): F[A] =
        ME.pure(x)
    }

}
