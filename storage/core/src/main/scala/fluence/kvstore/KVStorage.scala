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
import cats.syntax.flatMap._
import cats.{~>, Applicative, Monad, MonadError}
import fluence.codec.PureCodec
import fluence.kvstore.ops.Get.KVStoreGet
import fluence.kvstore.ops.Put.KVStorePut
import fluence.kvstore.ops.Remove.KVStoreRemove
import fluence.kvstore.ops.Traverse.KVStoreTraverse
import fluence.kvstore.ops.{Get, Put, Remove, Traverse}

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

  implicit def withCodecs[K, K1, V, V1](store: ReadWriteKVStore[K, V, StoreError])(
    implicit
    kCodec: PureCodec[K1, K],
    vCodec: PureCodec[V1, V]
  ): ReadWriteKVStore[K1, V1, StoreError] =
    new ReadWriteKVStore[K1, V1, StoreError] {

      /**
       * Returns lazy ''get'' representation (see [[Get]])
       *
       * @param key Search key
       */
      override def get(key: K1): Get[K1, V1, StoreError] = new Get[K1, V1, StoreError] {

        override def run[F[_]: Monad]: EitherT[F, StoreError, Option[V1]] =
          for {
            k ← kCodec.direct[F](key).leftMap(ce ⇒ StoreError(ce))
            v ← store.get(k).run
            v1 ← v match {
              case Some(v2) ⇒
                vCodec.inverse(v2).map(Option(_)).leftMap(ce ⇒ StoreError(ce))
              case None ⇒
                EitherT.right[StoreError](Monad[F].pure[Option[V1]](None))
            }
          } yield v1

        override def runUnsafe(): Option[V1] = {
          store
            .get(kCodec.direct.unsafe(key))
            .runUnsafe()
            .map(vCodec.inverse.unsafe)
        }

      }

      /**
       * Returns lazy ''traverse'' representation (see [[Traverse]])
       */
      override def traverse: Traverse[K1, V1, StoreError] = new Traverse[K1, V1, StoreError] {
        override def run[FS[_]: Monad](
          implicit FS: MonadError[FS, StoreError],
          liftIterator: ~>[Iterator, FS]
        ): FS[(K1, V1)] = {
          store.traverse.run.flatMap {
            case (k, v) ⇒
              val decodedPair = for {
                key ← kCodec.inverse(k)
                value ← vCodec.inverse(v)
              } yield key → value

              decodedPair.value.flatMap {
                case Right(pair) ⇒ FS.pure(pair)
                case Left(err) ⇒ FS.raiseError[(K1, V1)](StoreError(err))
              }
          }
        }

        override def runUnsafe: Iterator[(K1, V1)] =
          store.traverse.runUnsafe.map { case (k, v) ⇒ kCodec.inverse.unsafe(k) -> vCodec.inverse.unsafe(v) }

      }

      /**
       * Returns lazy ''put'' representation (see [[Put]])
       *
       * @param key   The specified key to be inserted
       * @param value The value associated with the specified key
       */
      override def put(key: K1, value: V1): Put[StoreError] = new Put[StoreError] {

        override def run[F[_]: Monad]: EitherT[F, StoreError, Unit] =
          for {
            k ← kCodec.direct[F](key).leftMap(ce ⇒ StoreError(ce))
            v ← vCodec.direct[F](value).leftMap(ce ⇒ StoreError(ce))
          } yield store.put(k, v).run

        override def runUnsafe(): Unit = {
          val k = kCodec.direct.unsafe(key)
          val v = vCodec.direct.unsafe(value)
          store.put(k, v).runUnsafe()
        }

      }

      /**
       * Returns lazy ''remove'' representation (see [[Remove]])
       *
       * @param key The specified key to be inserted
       */
      override def remove(key: K1): Remove[StoreError] = new Remove[StoreError] {

        override def run[F[_]: Monad]: EitherT[F, StoreError, Unit] =
          for {
            k ← kCodec.direct[F](key).leftMap(ce ⇒ StoreError(ce))
          } yield store.remove(k).run

        override def runUnsafe(): Unit =
          store.remove(kCodec.direct.unsafe(key)).runUnsafe()

      }
    }

}
