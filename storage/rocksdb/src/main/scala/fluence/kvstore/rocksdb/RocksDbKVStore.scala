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

package fluence.kvstore.rocksdb

import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.{~>, Monad}
import fluence.kvstore.KVStore.{GetOp, PutOp, RemoveOp, TraverseOp}
import fluence.kvstore.ops.{Operation, TraverseOperation}
import fluence.kvstore.rocksdb.RocksDbKVStore._
import fluence.kvstore.{Snapshotable, _}
import org.rocksdb.{Options, ReadOptions, RocksDB, RocksIterator}

import scala.concurrent.ExecutionContext
import scala.language.higherKinds

/**
 * Base thread-safe kvStore implementation based on RocksDb, that allow 'put',
 * 'remove' and 'get' by key and 'traverse'. Usage:
 *
 * {{{
 *    // get factory instance, which manage resources, you can create several
 *    // separated factories, but required single instance for application
 *    val factory = RocksDbKVStore.getFactory()
 *
 *    // create simple rockDb store
 *    val store: EitherT[IO, StoreError, RocksDbKVStore] = factory[IO](dbName, dbConfig, dbThreadPool)
 *
 *    // create rockDb store with snapshot possibility
 *    val store: EitherT[IO, StoreError, RocksDbKVStore with RocksDbSnapshotable] =
 *      factory.withSnapshots[IO](dbName, dbConfig, dbThreadPool)
 * }}}
 *
 * '''Note that''' RocksDb can store only binary data. For creating KVStore for
 * any type of key and value use [[KVStore.withCodecs()]] like this:
 *
 * {{{
 *   // rocksDb based binary store RocksDbKVStore ~ KVStore[Array[Byte], Array[Byte]]
 *   val binStore: RocksDbKVStore = ...
 *
 *   val store: ReadWriteKVStore[String, User]=
 *   KVStore.withCodecs(binStore)(
 *      bytes2StringCodec,
 *      bytes2UserCodec
 *   )
 * }}}
 */
class RocksDbKVStore(
  override protected val data: RocksDB,
  override protected val dbOptions: Options,
  override protected val kvStorePool: ExecutionContext,
  override protected val readOptions: ReadOptions = new ReadOptions()
) extends ReadWriteKVStore[Key, Value] with RocksDbKVStoreRead with RocksDbKVStoreWrite

object RocksDbKVStore {

  /**
   * Returns factory for creating RocksDbKVStore instances.
   * '''Note that''': each created RocksDb store is registered in factory.
   *
   * @param threadPool Default thread pool for each created instances.
   *                     Can be overridden in ''apply'' method.
   */
  def getFactory(threadPool: ExecutionContext = ExecutionContext.Implicits.global): RocksDbFactory =
    new RocksDbFactory(threadPool)

  type Key = Array[Byte]
  type Value = Array[Byte]

  /**
   * Top type for kvStore implementation based on RocksDb, it just holds kvStore state and meta information.
   */
  private[kvstore] sealed trait RocksDbKVStoreBase extends KVStore {

    /**
     * Java representation for c++ driver for RocksDb.
     */
    protected val data: RocksDB

    /**
     * Options to control common the behavior of a database.  It will be used
     * during the creation of a [[org.rocksdb.RocksDB]] (i.e., RocksDB.open())
     */
    protected val dbOptions: Options

    /**
     * Options that controls read behavior.
     */
    protected val readOptions: ReadOptions

    /**
     * Thread pool for perform all storage operations.
     */
    protected val kvStorePool: ExecutionContext

    /**
     * Users should always explicitly call close() methods for this entity!
     */
    def close(): IO[Unit] = IO {
      data.close()
      dbOptions.close()
      readOptions.close()
    }

  }

  /**
   * Allows reading keys and values from KVStore.
   */
  private[kvstore] trait RocksDbKVStoreRead extends RocksDbKVStoreBase with KVStoreRead[Key, Value] {

    /**
     * Returns lazy ''get'' representation (see [[Operation]])
     *
     * @param key Search key
     */
    override def get(key: Key): GetOp[Value] = new GetOp[Value] {

      override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Option[Value]] =
        EitherT {
          val io = IO.shift(kvStorePool) *> IO(Option(data.get(readOptions, key)))
          io.attempt.to[F]
        }.leftMap(err ⇒ StoreError.forGet(key, Some(err)))

    }

    /**
     * Returns lazy ''traverse'' representation (see [[TraverseOperation]])
     * Storage takes a implicit snapshot before make 'traverse' automatically.
     * Short-lived/foreground scans are best done via an 'traverse'
     * while long-running/background scans are better done via explicitly taking
     * 'snapshot' and invoking 'traverse' on the snapshot
     */
    override def traverse: TraverseOp[Key, Value] = new TraverseOp[Key, Value] {

      override def run[FS[_]: Monad: LiftIO](implicit liftIterator: Iterator ~> FS): FS[(Key, Value)] =
        newIterator().map(i ⇒ liftIterator(new RocksDbScalaIterator(i))).to[FS].flatten

      override def runUnsafe: Iterator[(Key, Value)] =
        newIterator().map(i ⇒ new RocksDbScalaIterator(i)).unsafeRunSync()

    }

    /**
     * Returns lazy operation for getting max stored 'key'.
     * '''Note''' returns not the last stored 'key', but the max key in natural order!
     * {{{
     *  For example:
     *    For numbers from 1 to 100, max key was 100.
     *    But for strings from k1 to k100, max key was k99, cause k100 < k99 in bytes representation
     * }}}
     */
    def getMaxKey: Operation[Option[Key]] = new Operation[Option[Key]] {

      override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Option[Key]] = {

        val result = {
          // 'tailing iterator' and 'seekToLast' isn't compatible options
          IO.shift(kvStorePool) *> newIterator(isTailing = false).bracket { iterator ⇒
            IO(iterator.seekToLast())
              .map[Option[Key]](_ ⇒ if (iterator.isValid) Option(iterator.key()) else None)
          } { iterator ⇒
            IO(iterator.close())
          }
        }.attempt.to[F]

        EitherT(result)
          .leftMap(err ⇒ StoreError("Can't get max key.", Some(err)))

      }

    }

    /**
     * Returns RocksDb iterator with resource managing.
     */
    private def newIterator(isTailing: Boolean = true): IO[RocksIterator] =
      IO.shift(kvStorePool) *> IO {

        /**
         * Tailing iterator needed for sequential read optimization, but it doesn't
         * take a snapshot when it's created and should not be used with Snapshotable,
         * see docs: [[https://github.com/facebook/rocksdb/wiki/Tailing-Iterator]]
         */
        if (readOptions.snapshot() == null)
          new ReadOptions(readOptions).setTailing(isTailing) // do optimisation
        else
          new ReadOptions(readOptions) // skip optimisation, because snapshot is taken

      }.bracket { opt ⇒
        IO(data.newIterator(opt))
      } { opt ⇒
        IO(opt.close()) // each RocksDb class should be closed for release the allocated memory in c++
      }

  }

  /**
   * Allows to create a point-in-time view of a storage.
   */
  private[kvstore] trait RocksDbSnapshotable extends RocksDbKVStoreBase with Snapshotable[RocksDbKVStoreRead] { self ⇒

    /**
     * Returns read-only key-value store snapshot with traverse functionality.
     * '''Note that''' you should invoke [[RocksDbKVStoreRead#close()]]
     * when current snapshot isn't needed anymore.
     *
     * If master RocksDbStore will be closed, every snapshots becomes in inconsistent state.
     * todo master kvstore should close all snapshots before it becomes closed
     */
    override def createSnapshot[F[_]: Monad: LiftIO]: F[RocksDbKVStoreRead] = {
      val newInstance: IO[RocksDbKVStoreRead] =
        for {
          snapshot ← IO.shift(kvStorePool) *> IO(self.data.getSnapshot)
          readOp ← IO(new ReadOptions(readOptions).setSnapshot(snapshot))
        } yield
          new RocksDbKVStoreBase with RocksDbKVStoreRead {
            override protected val data: RocksDB = self.data
            override protected val dbOptions: Options = self.dbOptions
            override protected val kvStorePool: ExecutionContext = self.kvStorePool
            override protected val readOptions: ReadOptions = readOp

            override def close(): IO[Unit] =
              IO(data.releaseSnapshot(snapshot))
          }

      newInstance.to[F]
    }
  }

  /**
   * Allows writing and removing keys and values from KVStore.
   */
  private[kvstore] trait RocksDbKVStoreWrite extends RocksDbKVStoreBase with KVStoreWrite[Key, Value] {

    /**
     * Returns lazy ''put'' representation (see [[Operation]])
     *
     * @param key The specified key to be inserted
     * @param value The value associated with the specified key
     */
    override def put(key: Key, value: Value): PutOp = new PutOp {

      override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Unit] =
        EitherT {
          val io = IO.shift(kvStorePool) *> IO(data.put(key, value))
          io.attempt.to[F]
        }.leftMap(err ⇒ StoreError.forPut(key, value, Some(err)))
          .map(_ ⇒ ())

    }

    /**
     * Returns lazy ''remove'' representation (see [[Operation]])
     * '''Note that concurrent writing is not supported!'''
     *
     * @param key A key to delete within database
     */
    override def remove(key: Key): RemoveOp = new RemoveOp {

      override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Unit] =
        EitherT {
          val io = IO.shift(kvStorePool) *> IO(data.delete(key))
          io.attempt.to[F]
        }.leftMap(err ⇒ StoreError.forRemove(key, Some(err)))
          .map(_ ⇒ ())

    }

  }

}
