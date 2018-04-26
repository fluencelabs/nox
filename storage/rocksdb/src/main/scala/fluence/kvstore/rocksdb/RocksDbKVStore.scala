package fluence.kvstore.rocksdb

import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.{~>, Eval, Monad}
import fluence.kvstore.KVStore.{GetOp, PutOp, RemoveOp, TraverseOp}
import fluence.kvstore.ops.{Operation, TraverseOperation}
import fluence.kvstore.rocksdb.RocksDbKVStore.{RocksDbKVStoreBase, RocksDbKVStoreGet, RocksDbKVStoreWrite}
import fluence.kvstore.{Snapshotable, _}
import fluence.storage.rocksdb.RocksDbScalaIterator
import monix.execution.Scheduler
import org.rocksdb.{Options, ReadOptions, RocksDB, RocksIterator}

import scala.language.higherKinds

/**
 * Base kvStore implementation based on RocksDb, that allow 'put', 'remove' and
 * 'get' by key. '''Note that''' RocksDb can store only binary data. For creating
 * KVStore for any type of key and value use [[KVStore.withCodecs()]] like this:
 * {{{
 *   todo write example, there may be special method in RocksDbFactory for comfort use
 * }}}
 */
class RocksDbKVStore(
  override protected val data: RocksDB,
  override protected val dbOptions: Options,
  override protected val kvStorePool: Scheduler,
  override protected val readOptions: ReadOptions = new ReadOptions()
) extends RocksDbKVStoreBase with RocksDbKVStoreGet with RocksDbKVStoreWrite

object RocksDbKVStore {

  /**
   * Returns single factory for creating RocksDbKVStore instances.
   * '''Note that''': every created RocksDb store is registered in global scope.
   * This method always returns the same global instance of RocksDbFactory.
   *
   * @param threadPool Default thread pool for each created instances.
   *                     Can be overridden in ''apply'' method.
   */
  def getFactory(threadPool: Scheduler = Scheduler.Implicits.global): Eval[RocksDbFactory] = {
    Eval.later(new RocksDbFactory(threadPool)).memoize
  }

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
     * The class that controls read behavior.
     */
    protected val readOptions: ReadOptions

    /**
     * Thread pool for perform all storage operations.
     */
    protected val kvStorePool: Scheduler

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
  private[kvstore] trait RocksDbKVStoreRead
      extends RocksDbKVStoreGet with RocksDbKVStoreTraverse with KVStoreRead[Key, Value]

  /**
   * Allows getting values from KVStore by the key.
   */
  private[kvstore] trait RocksDbKVStoreGet extends RocksDbKVStoreBase with KVStoreGet[Key, Value] {

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

  }
  // todo merge get and traverse implementations
  /**
   * Allows to 'traverse' KVStore keys-values pairs.
   * '''Note that''', 'traverse' method appears only after taking snapshot.
   */
  private[kvstore] trait RocksDbKVStoreTraverse extends RocksDbKVStoreBase with KVStoreTraverse[Key, Value] {

    /**
     * Returns lazy ''traverse'' representation (see [[TraverseOperation]])
     */
    override def traverse: TraverseOp[Key, Value] = new TraverseOp[Key, Value] {

      override def run[FS[_]: Monad: LiftIO](implicit liftIterator: Iterator ~> FS): FS[(Key, Value)] =
        newIterator.map(i ⇒ liftIterator(new RocksDbScalaIterator(i))).to[FS].flatten

      override def runUnsafe: Iterator[(Key, Value)] =
        newIterator.map(i ⇒ new RocksDbScalaIterator(i)) unsafeRunSync ()

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
          IO.shift(kvStorePool) *> newIterator.bracket { iterator ⇒
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
    private def newIterator: IO[RocksIterator] =
      IO.shift(kvStorePool) *> IO {

        /**
         * Tailing iterator needed for sequential read optimization, but it doesn't
         * take a snapshot when it's created and should not be used with Snapshotable,
         * see docs: [[https://github.com/facebook/rocksdb/wiki/Tailing-Iterator]]
         */
        if (readOptions.snapshot() == null)
          new ReadOptions(readOptions).setTailing(true) // do optimisation
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
    override def createSnapshot[F[_]: LiftIO]: F[RocksDbKVStoreRead] = {
      val newInstance: IO[RocksDbKVStoreRead] =
        for {
          snapshot ← IO.shift(kvStorePool) *> IO(self.data.getSnapshot)
          readOp ← IO(new ReadOptions(readOptions).setSnapshot(snapshot))
        } yield
          new RocksDbKVStoreBase with RocksDbKVStoreRead {
            override protected val data: RocksDB = self.data
            override protected val dbOptions: Options = self.dbOptions
            override protected val readOptions: ReadOptions = readOp
            override protected val kvStorePool: Scheduler = self.kvStorePool

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
