package fluence.kvstore.rocksdb

import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import cats.syntax.flatMap._
import cats.{~>, Monad}
import fluence.kvstore.KVStore.TraverseOp
import fluence.kvstore.ops.{Operation, TraverseOperation}
import fluence.kvstore.rocksdb.RocksDbKVStore.{RocksDbKVStoreGet, RocksDbStoreBaseImpl}
import fluence.kvstore.{Snapshotable, _}
import fluence.storage.rocksdb.RocksDbScalaIterator
import org.rocksdb.{Options, ReadOptions, RocksDB}

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
  private val db: RocksDB,
  private val dbOps: Options,
  private val readOpt: ReadOptions = new ReadOptions()
) extends RocksDbStoreBaseImpl(db, dbOps, readOpt) with RocksDbKVStoreGet /*with RocksDbKVStoreWrite*/

// todo there need a special ThreadPool for IO operations in RocksDb
object RocksDbKVStore {

  lazy val create: RocksDbFactory = new RocksDbFactory

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
    override def get(key: Key): Operation[Option[Value]] = new Operation[Option[Value]] {

      override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Option[Value]] =
        EitherT(IO(Option(data.get(readOptions, key))).attempt.to[F])
          .leftMap(err ⇒ StoreError.forGet(key, Some(err)))

    }

  }

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
        IO {
          new ReadOptions(readOptions).setTailing(true) // sequential read optimization
        }.bracket { opt ⇒
          IO(liftIterator(new RocksDbScalaIterator(data.newIterator(opt))))
        } { opt ⇒
          IO(opt.close()) // each RocksDb class should be closed for release the allocated memory in c++
        }.to[FS].flatten

      override def runUnsafe: Iterator[(Key, Value)] =
        new RocksDbScalaIterator(data.newIterator)

    }

  }

  /**
   * Allows to taking snapshot for the RocksDbKVStore.
   */
  private[kvstore] trait RocksDbSnapshotable extends RocksDbKVStoreBase with Snapshotable[RocksDbKVStoreRead] { self ⇒

    /**
     * Returns read-only key-value store snapshot with traverse functionality.
     */
    override def createSnapshot[F[_]: LiftIO](): F[RocksDbKVStoreRead] = {
      val newInstance: IO[RocksDbKVStoreRead] =
        for {
          snapshot ← IO(self.data.getSnapshot)
          readOp ← IO(new ReadOptions(readOptions).setSnapshot(snapshot))
        } yield
          new RocksDbStoreBaseImpl(self.data, self.dbOptions, readOp) with RocksDbKVStoreRead {
            override def close(): Unit = {
              super.close()
              readOp.close()
              snapshot.close()
            }
          }

      newInstance.to[F]
    }
  }

  // todo implement RocksDbKVStoreWrite

  // todo create 2 apply methods: for binary store and store with codecs

  // todo try to avoid double convert when F is Task

  /**
   * Implementation of KVStore inner state holder, based on RocksDb java driver.
   */
  private[kvstore] class RocksDbStoreBaseImpl(
    override val data: RocksDB,
    override val dbOptions: Options,
    override val readOptions: ReadOptions
  ) extends RocksDbKVStoreBase with AutoCloseable {

    // todo consider more functional interface for closing resources with IO
    /**
     * Users should always explicitly call close() methods for this entity!
     */
    override def close(): Unit = {
      data.close()
      dbOptions.close()
      readOptions.close()
    }

  }

}
