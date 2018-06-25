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
import cats.syntax.functor._
import cats.{~>, Monad}
import fluence.codec.PureCodec
import fluence.kvstore.KVStore.{GetOp, PutOp, RemoveOp, TraverseOp}
import fluence.kvstore.ops._

import scala.language.higherKinds

/**
 * Top type for any key value storage.
 */
trait KVStore

/**
 * Key-value storage api for reading values.
 *
 * @tparam K The type of keys
 * @tparam V The type of stored values
 */
trait KVStoreRead[K, V] extends KVStore {

  /**
   * Returns lazy ''get'' representation (see [[Operation]])
   *
   * @param key Search key
   */
  def get(key: K): GetOp[V]

  /**
   * Returns lazy ''traverse'' representation (see [[TraverseOperation]])
   * '''Note that''', 'traverse' without taking snapshot can lead to non
   * deterministic behavior.
   */
  def traverse: TraverseOp[K, V]

}

/**
 * Key-value storage api for writing values.
 *
 * @tparam K The type of keys
 * @tparam V The type of stored values
 */
trait KVStoreWrite[K, V] extends KVStore {

  /**
   * Returns lazy ''put'' representation (see [[Operation]])
   *
   * @param key The specified key to be inserted
   * @param value The value associated with the specified key
   */
  def put(key: K, value: V): PutOp

  /**
   * Returns lazy ''remove'' representation (see [[Operation]])
   *
   * @param key A key to delete within database
   */
  def remove(key: K): RemoveOp

}

/**
 * Key-value storage api for reading and writing.
 *
 * @tparam K The type of keys
 * @tparam V The type of stored values
 */
trait ReadWriteKVStore[K, V] extends KVStoreRead[K, V] with KVStoreWrite[K, V]

/**
 * Allows to create a point-in-time view of a storage.
 *
 * @tparam S The type of returned storage snapshot.
 */
trait Snapshotable[+S <: KVStoreRead[_, _]] {

  /**
   * Returns read-only key-value store snapshot.
   */
  def createSnapshot[F[+ _]: Monad: LiftIO]: F[S]

}

object KVStore {

  type GetOp[V] = Operation[Option[V]]
  type TraverseOp[K, V] = TraverseOperation[K, V]
  type PutOp = Operation[Unit]
  type RemoveOp = Operation[Unit]

  /**
   * Wraps simple storage into codecs.
   *
   * @param store Origin storage for wrapping
   * @param kCodec Codec for key from type 'K' to 'K1'
   * @param vCodec Codec for value from type 'V' to 'V1'
   */
  implicit def withCodecs[K, K1, V, V1](store: ReadWriteKVStore[K, V])(
    implicit
    kCodec: PureCodec[K1, K],
    vCodec: PureCodec[V1, V]
  ): ReadWriteKVStore[K1, V1] =
    new WrappedReadWriteKVStore(store)

  /**
   * Wraps snapshotable storage into codecs.
   *
   * @param store Origin storage for wrapping
   * @param kCodec Codec for key from type 'K' to type 'K1'
   * @param vCodec Codec for value from type 'K' to type 'K1'
   */
  implicit def withCodecsForSnapshotable[K, K1, V, V1](
    store: ReadWriteKVStore[K, V] with Snapshotable[KVStoreRead[K, V]]
  )(
    implicit
    kCodec: PureCodec[K1, K],
    vCodec: PureCodec[V1, V]
  ): ReadWriteKVStore[K1, V1] with Snapshotable[KVStoreRead[K1, V1]] =
    new WrappedReadWriteKVStore(store) with Snapshotable[KVStoreRead[K1, V1]] {
      override def createSnapshot[F[+ _]: Monad: LiftIO]: F[KVStoreRead[K1, V1]] = {
        val value: F[KVStoreRead[K1, V1]] =
          store.createSnapshot[F].map(new WrappedReadKVStore(_))
        value
      }
    }

  /**
   * Wrapper into codecs for a read storage.
   */
  private class WrappedReadKVStore[K, V, K1, V1](
    originStore: KVStoreRead[K, V]
  )(
    implicit
    kCodec: PureCodec[K1, K],
    vCodec: PureCodec[V1, V]
  ) extends KVStoreRead[K1, V1] {

    /**
     * Returns lazy ''get'' representation (see [[Operation]])
     *
     * @param key Search key
     */
    override def get(key: K1): GetOp[V1] = new GetOp[V1] {

      override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Option[V1]] =
        for {
          k ← kCodec.direct[F](key).leftMap(StoreError(_))
          v ← originStore.get(k).run
          v1 ← v match {
            case Some(v2) ⇒
              vCodec.inverse(v2).map(Option(_)).leftMap(StoreError(_))
            case None ⇒
              EitherT.rightT[F, StoreError](None)
          }
        } yield v1

    }

    /**
     * Returns lazy ''traverse'' representation (see [[TraverseOperation]])
     */
    override def traverse: TraverseOp[K1, V1] = new TraverseOp[K1, V1] {
      override def run[FS[_]: Monad: LiftIO](
        implicit liftIterator: Iterator ~> FS
      ): FS[(K1, V1)] =
        originStore.traverse.run.flatMap {
          case (k, v) ⇒
            val decodedPair = for {
              key ← kCodec.inverse(k)
              value ← vCodec.inverse(v)
            } yield key → value

            decodedPair.value.flatMap {
              case Right(pair) ⇒ Monad[FS].pure(pair)
              case Left(err) ⇒ IO.raiseError[(K1, V1)](StoreError(err)).to[FS]
            }
        }

      override def runUnsafe: Iterator[(K1, V1)] =
        originStore.traverse.runUnsafe.map { case (k, v) ⇒ kCodec.inverse.unsafe(k) -> vCodec.inverse.unsafe(v) }

    }
  }

  /**
   * Wrapper into codecs for a read-write storage.
   */
  private class WrappedReadWriteKVStore[K, V, K1, V1](
    originStore: ReadWriteKVStore[K, V]
  )(
    implicit
    kCodec: PureCodec[K1, K],
    vCodec: PureCodec[V1, V]
  ) extends WrappedReadKVStore[K, V, K1, V1](originStore) with ReadWriteKVStore[K1, V1] {

    /**
     * Returns lazy ''put'' representation (see [[Operation]])
     *
     * @param key   The specified key to be inserted
     * @param value The value associated with the specified key
     */
    override def put(key: K1, value: V1): PutOp = new PutOp {

      override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Unit] =
        for {
          k ← kCodec.direct[F](key).leftMap(StoreError(_))
          v ← vCodec.direct[F](value).leftMap(StoreError(_))
          r ← originStore.put(k, v).run
        } yield r

    }

    /**
     * Returns lazy ''remove'' representation (see [[Operation]])
     *
     * @param key A key to delete within database
     */
    override def remove(key: K1): RemoveOp = new RemoveOp {

      override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Unit] =
        for {
          k ← kCodec.direct[F](key).leftMap(StoreError(_))
          r ← originStore.remove(k).run
        } yield r

    }
  }

}
