package fluence.kvstore.rocksdb

import java.io.File

import cats.Monad
import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import com.typesafe.config.Config
import fluence.kvstore.StoreError
import fluence.kvstore.rocksdb.RocksDbKVStore._
import monix.execution.atomic.AtomicBoolean
import org.rocksdb.{Options, RocksDB}

import scala.collection.concurrent.TrieMap
import scala.language.higherKinds

// todo don't forget about scheduler for RocksDbStore
// todo write unit test

/**
 * Factory should be used to create all the instances of RocksDbKVStore
 * todo discuss, global state isn't good approach, needed to consider consider another way
 */
private[kvstore] class RocksDbFactory extends slogging.LazyLogging {

  private val isClosed = IO.pure(AtomicBoolean(false))
  private val instances = IO.pure(TrieMap.empty[String, RocksDbKVStore])

  /**
   * Creates RocksDb instance for specified name.
   * All data will be stored in {{{ s"${RocksDbConf.dataDir}/storeName" }}}.
   *
   * @param storeName The name of current RocksDbStore instance
   * @param conf       TypeSafe config
   */
  def apply[F[_]: Monad: LiftIO](storeName: String, conf: Config): EitherT[F, StoreError, RocksDbKVStore] =
    apply(storeName, conf, new RocksDbKVStore(_, _))

  /**
   * Creates RocksDb instance for specified name with snapshot and traverse
   * functionality. All data will be stored in {{{ s"${RocksDbConf.dataDir}/storeName" }}}.
   *
   * @param storeName The name of current RocksDbStore instance
   * @param conf       TypeSafe config
   */
  def withSnapshots[F[_]: Monad: LiftIO](
    storeName: String,
    conf: Config
  ): EitherT[F, StoreError, RocksDbKVStore with RocksDbSnapshotable] =
    apply(storeName, conf, new RocksDbKVStore(_, _) with RocksDbSnapshotable)

  /**
   * Closes all launched instances of RocksDB
   */
  def close: IO[Unit] =
    for {
      _ ← isClosed.map(_.set(true))
      allInstances ← instances
    } yield {
      logger.info(s"Closing RocksDB instances: ${allInstances.keys.mkString(", ")}")
      val set =
        allInstances.keySet
          .flatMap(ds ⇒ allInstances.remove(ds))
          .map(_.close())
          .fold(IO.unit)(_)
    }

  /* Utils methods */

  /**
   * Main method for creating each os RocksDb instance, checks that this Factory
   * isn't closed yet.
   */
  private def apply[F[_]: Monad: LiftIO, S <: RocksDbKVStore](
    storeName: String,
    conf: Config,
    newInstance: (RocksDB, Options) ⇒ S
  ): EitherT[F, StoreError, S] =
    for {
      _ ← checkClose
      conf ← RocksDbConf.read[F](conf)
      store ← EitherT(
        createKVStore(storeName, conf, newInstance).attempt.to[F]
      ).leftMap(err ⇒ StoreError(s"RocksDb initialisation exception for storeName=$storeName", Some(err)))
    } yield {
      store
    }

  private def createKVStore[S <: RocksDbKVStore](
    storeName: String,
    config: RocksDbConf,
    newInstance: (RocksDB, Options) ⇒ S
  ): IO[S] =
    for {
      options ← createOptionsFromConfig(config)
      dbRoot = s"${config.dataDir}/$storeName"
      nativeRocksDb ← createNativeDb(dbRoot, options)
      rocksDbKVStore ← registeringInstance(storeName, newInstance, options, nativeRocksDb)
    } yield rocksDbKVStore

  private def registeringInstance[S <: RocksDbKVStore](
    storeName: String,
    newInstance: (RocksDB, Options) ⇒ S,
    options: Options,
    rdb: RocksDB
  ): IO[S] = {
    instances.map { allInstances ⇒
      val store = newInstance(rdb, options)
      allInstances.put(storeName, store)
      logger.info(s"RocksDB instance created for $storeName")
      store
    }
  }

  private def createNativeDb(folder: String, options: Options): IO[RocksDB] =
    IO {
      RocksDB.loadLibrary()
      val dataDir = new File(folder)
      if (!dataDir.exists()) dataDir.mkdirs()
      RocksDB.open(options, folder)
    }

  private def checkClose[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Unit] = {
    EitherT(isClosed.attempt.to[F])
      .leftMap(StoreError(_))
      .map(_.get)
      .flatMap {
        case true ⇒
          EitherT.leftT[F, Unit](StoreError("Can't perform anything, because RocksDbFactory was closed"))
        case false ⇒
          EitherT.rightT[F, StoreError](())
      }
  }

  private def createOptionsFromConfig(conf: RocksDbConf): IO[Options] =
    IO {
      val opt = new Options()
      opt.setCreateIfMissing(conf.createIfMissing)
      opt
    }

}
