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

package fluence.storage.rocksdb

import java.io.File

import com.typesafe.config.{ Config, ConfigFactory }
import RocksDbStore._
import cats.effect.IO
import cats.{ ApplicativeError, MonadError }
import cats.syntax.functor._
import cats.syntax.flatMap._
import fluence.storage.{ KVStore, TraversableKVStore }
import monix.eval.{ Task, TaskSemaphore }
import monix.reactive.Observable
import org.rocksdb.{ Options, ReadOptions, RocksDB }

import scala.collection.concurrent.TrieMap
import scala.reflect.io.Path
import scala.language.higherKinds

/**
 * Implementation of [[KVStore]] with embedded RocksDB database.
 * For each dataSet will be created new RocksDB instance in a separate folder.
 * All defaults for all instances are stored in the typeSafe config (see ''reference.conf''),
 * you can overload them into ''application.conf''
 * '''Note: only single thread should write to DB''', but reading database allowed to multiply threads.
 *
 * @param dataSet   Some bag for pairs (K, V)
 * @param db        Instance of RocksDbJava driver
 * @param dbOptions Needed for run [[dbOptions.close]] simultaneously with [[db.close]].
 */
class RocksDbStore(
    val dataSet: String,
    val db: RocksDB,
    private val dbOptions: Options
) extends KVStore[Task, Key, Value] with TraversableKVStore[Observable, Key, Value] with AutoCloseable {

  private val writeMutex = TaskSemaphore(1)

  // todo logging

  /**
   * Gets stored value for specified key.
   *
   * @param key The key retrieve the value.
   */
  override def get(key: Key): Task[Value] =
    Task.eval(Option(db.get(key))).flatMap {
      case Some(v) ⇒ Task.now(v)
      case None    ⇒ Task.raiseError(KVStore.KeyNotFound)
    }

  /**
   * Puts key value pair (K, V). Put is synchronous operation.
   * '''Note that concurrent writing is not supported!'''
   *
   * @param key   the specified key to be inserted
   * @param value the value associated with the specified key
   */
  override def put(key: Key, value: Value): Task[Unit] = {
    writeMutex.greenLight(Task(db.put(key, value)))
  }

  /**
   * Removes pair (K, V) for specified key.
   * '''Note that concurrent writing is not supported!'''
   *
   * @param key Key to delete within database
   */
  override def remove(key: Key): Task[Unit] = {
    writeMutex.greenLight(Task(db.delete(key)))
  }

  /**
   * Return all pairs (K, V) for specified dataSet.
   *
   * @return Cursor for found pairs (K,V)
   */
  override def traverse(): Observable[(Key, Value)] = {

    lazy val snapshot = db.getSnapshot
    lazy val options = new ReadOptions()

    Observable(())
      .doOnSubscribe { () ⇒
        options.setSnapshot(snapshot) // take a snapshot only when subscribing appears
        options.setTailing(true) // sequential read optimization
      }
      .doAfterTerminate { _ ⇒
        db.releaseSnapshot(snapshot)
        snapshot.close()
        options.close()
      }
      .flatMap(_ ⇒ Observable.fromIterator(new RocksDbScalaIterator(db.newIterator(options))))

  }

  /** Users should always explicitly call close() methods for this entity! */
  override def close(): Unit = {
    db.close()
    dbOptions.close()
  }

}

object RocksDbStore {

  type Key = Array[Byte]
  type Value = Array[Byte]

  /**
   * Factory should be used to create all the instances of RocksDbStore
   */
  class Factory extends slogging.LazyLogging {
    private val instances = TrieMap.empty[String, RocksDbStore]

    def apply[F[_]](dataSet: String)(implicit F: MonadError[F, Throwable]): F[RocksDbStore] =
      apply(dataSet, ConfigFactory.load())

    def apply[F[_]](dataSet: String, conf: Config)(implicit F: MonadError[F, Throwable]): F[RocksDbStore] =
      RocksDbConf.read(conf).flatMap(apply(dataSet, _))

    def apply[F[_]](dataSet: String, config: RocksDbConf)(implicit F: ApplicativeError[F, Throwable]): F[RocksDbStore] = {
      val dbRoot = s"${config.dataDir}/$dataSet"
      val options = createOptionsFromConfig(config, Path(dbRoot))

      createDb(dbRoot, options)
        .map{ rdb ⇒
          // Registering an instance
          val store = new RocksDbStore(dataSet, rdb, options)
          instances(dataSet) = store
          logger.info(s"RocksDB instance created for $dataSet")
          store
        }
    }

    /**
     * Closes all launched instances of RocksDB
     */
    def close(): IO[Unit] = IO {
      logger.info(s"Closing RocksDB instances: ${instances.keys.mkString(", ")}")
      instances.keySet.flatMap(ds ⇒ instances.remove(ds)).foreach(_.close())
    }

    private def createDb[F[_]](folder: String, options: Options)(implicit F: ApplicativeError[F, Throwable]): F[RocksDB] =
      F.catchNonFatal {
        RocksDB.loadLibrary()
        val dataDir = new File(folder)
        if (!dataDir.exists()) dataDir.mkdirs()
        RocksDB.open(options, folder)
      }

    private def createOptionsFromConfig(conf: RocksDbConf, root: Path): Options = {
      val opt = new Options()
      opt.setCreateIfMissing(conf.createIfMissing)
      opt
    }
  }
}

case class RocksDbConf(dataDir: String, createIfMissing: Boolean)

object RocksDbConf {
  val ConfigPath = "fluence.node.storage.rocksDb"

  def read[F[_]](conf: Config, name: String = ConfigPath)(implicit F: ApplicativeError[F, Throwable]): F[RocksDbConf] =
    F.catchNonFatal{
      import net.ceedubs.ficus.Ficus._
      import net.ceedubs.ficus.readers.ArbitraryTypeReader._
      conf.as[RocksDbConf](name)
    }
}
