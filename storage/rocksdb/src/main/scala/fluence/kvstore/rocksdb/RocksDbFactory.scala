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
import scala.concurrent.ExecutionContext
import scala.language.higherKinds

// todo: unit test
// todo: write examples of creating different instances of RocksDb
// todo create apply method: for binary store and store with codecs

/**
 * This factory should be used to create every instances of RocksDbKVStore.
 * This factory registers all created instances for possibility to close its all
 * in one place, see [[RocksDbFactory#close()]].
 *
 * @param defaultPool Default thread pool for each created instances.
 *                      Can be overridden in ''apply'' method.
 */
// todo discuss, global state isn't good approach, needed to consider another way
private[kvstore] class RocksDbFactory(defaultPool: ExecutionContext) extends slogging.LazyLogging {

  private val isClosed = IO.pure(AtomicBoolean(false))
  private val instances = IO.pure(TrieMap.empty[String, RocksDbKVStore])

  /**
   * Creates RocksDb instance for specified name.
   * All data will be stored in {{{ s"${RocksDbConf.dataDir}/storeName" }}}.
   *
   * @param storeName The name of current RocksDbStore instance
   * @param conf       TypeSafe config
   * @param threadPool All operations of created RocksDbKVStore will be perform
   *                     on this thread pool
   */
  def apply[F[_]: Monad: LiftIO](
    storeName: String,
    conf: Config,
    threadPool: ExecutionContext = defaultPool
  ): EitherT[F, StoreError, RocksDbKVStore] =
    apply(storeName, conf, new RocksDbKVStore(_, _, defaultPool))

  /**
   * Creates RocksDb instance for specified name with snapshot and traverse
   * functionality. All data will be stored in {{{ s"${RocksDbConf.dataDir}/storeName" }}}.
   *
   * @param storeName The name of current RocksDbStore instance
   * @param conf       TypeSafe config
   * @param threadPool All operations of created RocksDbKVStore will be perform
   *                     on this thread pool
   */
  def withSnapshots[F[_]: Monad: LiftIO](
    storeName: String,
    conf: Config,
    threadPool: ExecutionContext = defaultPool
  ): EitherT[F, StoreError, RocksDbKVStore with RocksDbSnapshotable] =
    apply(storeName, conf, new RocksDbKVStore(_, _, defaultPool) with RocksDbSnapshotable)

  /**
   * Closes all launched instances of RocksDB.
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
