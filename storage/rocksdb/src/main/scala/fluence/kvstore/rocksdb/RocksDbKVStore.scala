package fluence.kvstore.rocksdb

import java.io.File

import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{~>, ApplicativeError, Monad, MonadError}
import com.typesafe.config.Config
import fluence.kvstore.KVStore.TraverseOp
import fluence.kvstore._
import fluence.kvstore.ops.{Operation, TraverseOperation}
import fluence.kvstore.rocksdb.RocksDbKVStore.{RocksDbKVStoreGet, RocksDbStoreBaseImpl}
import fluence.storage.rocksdb.{RocksDbConf, RocksDbScalaIterator}
import org.rocksdb.{Options, ReadOptions, RocksDB}

import scala.collection.concurrent.TrieMap
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
  private val dbOps: Options
) extends RocksDbStoreBaseImpl(db, dbOps) with RocksDbKVStoreGet /*with RocksDbKVStoreWrite*/

object RocksDbKVStore {

  lazy val factory: RocksDbKVStore.Factory = new RocksDbKVStore.Factory

  // todo there need a special ThreadPool for IO operations in RocksDb

  type Key = Array[Byte]
  type Value = Array[Byte]

  /**
   * Top type for kvStore implementation based on RocksDb, it just holds kvStore state.
   */
  private[kvstore] sealed trait RocksDbKVStoreBase extends KVStore {

    protected val data: RocksDB

  }

  /**
   * Allows reading keys and values from KVStore.
   */
  private trait RocksDbKVStoreRead extends RocksDbKVStoreGet with RocksDbKVStoreTraverse with KVStoreRead[Key, Value]

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
        EitherT(IO(Option(data.get(key))).attempt.to[F])
          .leftMap(err ⇒ StoreError.forGet(key, Some(err)))

    }

  }

  /**
   * Allows to 'traverse' KVStore keys-values pairs.
   * '''Note that''', 'traverse' method appears only after taking snapshot.
   */
  private trait RocksDbKVStoreTraverse extends RocksDbKVStoreBase with KVStoreTraverse[Key, Value] {

    /**
     * Returns lazy ''traverse'' representation (see [[TraverseOperation]])
     */
    override def traverse: TraverseOp[Key, Value] = new TraverseOp[Key, Value] {

      override def run[FS[_]: Monad: LiftIO](implicit liftIterator: Iterator ~> FS): FS[(Key, Value)] =
        IO {
          new ReadOptions().setTailing(true) // sequential read optimization
        }.bracket { opt ⇒
          IO(liftIterator(new RocksDbScalaIterator(data.newIterator(opt))))
        } { opt ⇒
          IO(opt.close()) // each RocksDb class should be closed for release the allocated memory in c++
        }.to[FS].flatten

      override def runUnsafe: Iterator[(Key, Value)] =
        new RocksDbScalaIterator(data.newIterator)

    }

  }

  // todo implement RocksDbKVStoreWrite

  // todo create 2 apply methods: for binary store and store with codecs

  // todo try to avoid double convert when F is Task

  /**
   * Implementation of KVStore inner state holder, based on RocksDb java driver.
   */
  private[kvstore] abstract class RocksDbStoreBaseImpl(
    private val db: RocksDB,
    private val dbOptions: Options
  ) extends RocksDbKVStoreBase with AutoCloseable {

    override protected val data: RocksDB = db

    /**
     * Users should always explicitly call close() methods for this entity!
     */
    override def close(): Unit = {
      db.close()
      dbOptions.close()
    }

  }

  // todo finish @@@@ don't forget about scheduler
  /**
   * Factory should be used to create all the instances of RocksDbKVStore
   * todo discuss, global state isn't good approach, needed to consider consider another way
   */
  private[kvstore] class Factory extends slogging.LazyLogging {
    private val instances = TrieMap.empty[String, RocksDbKVStore]

    /**
     * Create RocksDb instance for specified name.
     * All data will be stored in {{{ s"${RocksDbConf.dataDir}/storeName" }}}.
     *
     * @param storeName The name of current RocksDbStore instance
     * @param conf       TypeSafe config
     */

    def apply[F[_]](storeName: String, conf: Config)(implicit F: MonadError[F, Throwable]): F[RocksDbKVStore] =
      RocksDbConf.read(conf).flatMap(apply(storeName, _))

    def apply[F[_]](storeName: String, config: RocksDbConf)(
      implicit F: ApplicativeError[F, Throwable]
    ): F[RocksDbKVStore] = {
      val dbRoot = s"${config.dataDir}/$storeName"
      val options = createOptionsFromConfig(config)

      createDb(dbRoot, options).map { rdb ⇒
        // Registering an instance
        val store = new RocksDbKVStore(rdb, options)
        instances(storeName) = store
        logger.info(s"RocksDB instance created for $storeName")
        store
      }
    }

    /**
     * Closes all launched instances of RocksDB
     */
    def close: IO[Unit] = IO {
      logger.info(s"Closing RocksDB instances: ${instances.keys.mkString(", ")}")
      instances.keySet.flatMap(ds ⇒ instances.remove(ds)).foreach(_.close())
    }

    private def createDb[F[_]](folder: String, options: Options)(
      implicit F: ApplicativeError[F, Throwable]
    ): F[RocksDB] =
      F.catchNonFatal {
        RocksDB.loadLibrary()
        val dataDir = new File(folder)
        if (!dataDir.exists()) dataDir.mkdirs()
        RocksDB.open(options, folder)
      }

    private def createOptionsFromConfig(conf: RocksDbConf): Options = {
      val opt = new Options()
      opt.setCreateIfMissing(conf.createIfMissing)
      opt
    }
  }
}
