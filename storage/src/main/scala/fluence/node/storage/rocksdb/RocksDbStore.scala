package fluence.node.storage.rocksdb

import java.io.File

import com.typesafe.config.{ Config, ConfigFactory }
import fluence.node.storage.KVStore
import fluence.node.storage.rocksdb.RocksDbStore._
import monix.eval.{ Task, TaskSemaphore }
import monix.reactive.Observable
import org.rocksdb.{ Options, ReadOptions, RocksDB }

import scala.reflect.io.Path
import scala.util.Try

/**
 * Implementation of [[KVStore]] with embedded RocksDB database.
 * For each dataSet will be created new RocksDB instance in a separate folder.
 * All defaults for all instances are stored in the typeSafe config (see ''reference.conf''),
 * you can overload them into ''application.conf''
 * '''Note: only single thread should write to DB''', but reading database allowed to multiply threads.
 *
 * @param dataSet   some bag for pairs (K, V)
 * @param db        instance of RocksDbJava driver
 * @param dbOptions needed for run [[dbOptions.close]] simultaneously with [[db.close]].
 */
class RocksDbStore(
    val dataSet: String,
    val db: RocksDB,
    private val dbOptions: Options
) extends KVStore[Key, Value, Task, Observable] with AutoCloseable {

  private val writeMutex = TaskSemaphore(1)

  // todo logging

  /**
   * Gets stored value for specified key.
   *
   * @param key the key retrieve the value.
   */
  override def get(key: Key): Task[Value] = {
    Task(db.get(key))
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
   * @param key key to delete within database
   */
  override def remove(key: Key): Task[Unit] = {
    writeMutex.greenLight(Task(db.delete(key)))
  }

  /**
   * Return all pairs (K, V) for specified dataSet.
   *
   * @return cursor of founded pairs (K,V)
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

  def apply(dataSet: String): Try[RocksDbStore] =
    apply(dataSet)

  def apply(dataSet: String, conf: Config): Try[RocksDbStore] =
    apply(dataSet, RocksDbConf.read(conf = conf))

  def apply(dataSet: String, config: RocksDbConf): Try[RocksDbStore] = {
    val dbRoot = s"${config.dataDir}/$dataSet"
    val options = createOptionsFromConfig(config, Path(dbRoot))

    createDb(dbRoot, options)
      .map(new RocksDbStore(dataSet, _, options))
  }

  private def createDb(folder: String, options: Options): Try[RocksDB] = {
    Try {
      RocksDB.loadLibrary()
      val dataDir = new File(folder)
      if (!dataDir.exists()) dataDir.mkdirs()
      RocksDB.open(options, folder)
    }
  }

  private def createOptionsFromConfig(conf: RocksDbConf, root: Path): Options = {
    val opt = new Options()
    opt.setCreateIfMissing(conf.createIfMissing)
    opt
  }
}

case class RocksDbConf(dataDir: String, createIfMissing: Boolean)

object RocksDbConf {
  val ConfigPath = "fluence.node.storage.rocksDb"

  def read(name: String = ConfigPath, conf: Config = ConfigFactory.load()): RocksDbConf = {
    import net.ceedubs.ficus.Ficus._
    import net.ceedubs.ficus.readers.ArbitraryTypeReader._
    conf.as[RocksDbConf](name)
  }
}
