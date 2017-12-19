package fluence.node.storage

import java.nio.ByteBuffer
import java.util.concurrent.ConcurrentHashMap

import fluence.node.storage.InMemoryKVStore._
import monix.eval.Task
import monix.reactive.Observable

import scala.collection.JavaConverters._
import scala.collection.concurrent

/**
 * In memory implementation of [[KVStore]].
 * The keys are wrapped around ByteBuffer for valid hashCode() and equals methods.
 * TODO: consider either moving to tests folder, or replacing with [[TrieMapKVStore]]
 */
class InMemoryKVStore(db: concurrent.Map[ByteBuffer, Value])
  extends KVStore[Task, Key, Value] with TraversableKVStore[Observable, Key, Value] {

  /**
   * Gets stored value for specified key.
   *
   * @param key the key retrieve the value.
   */
  override def get(key: Key): Task[Value] = {
    Task.delay(db.get(key)).flatMap {
      case Some(v) ⇒ Task.now(v)
      case None    ⇒ Task.raiseError(KVStore.KeyNotFound)
    }
  }

  /**
   * Puts key value pair (K, V). Put is synchronous operation.
   *
   * @param key   the specified key to be inserted
   * @param value the value associated with the specified key
   */
  override def put(key: Key, value: Value): Task[Unit] = {
    Task(db.put(key, value))
  }

  /**
   * Removes pair (K, V) for specified key.
   *
   * @param key key to delete within database
   */
  override def remove(key: Key): Task[Unit] = {
    Task(db.remove(key))
  }

  /**
   * Return all pairs (K, V) for specified dataSet.
   *
   * @return cursor of founded pairs (K,V)
   */
  override def traverse(): Observable[(Key, Value)] = {
    Observable.fromIterator(db.iterator.map { case (k, v) ⇒ k.array() → v })
  }

}

object InMemoryKVStore {

  type Key = Array[Byte]
  type Value = Array[Byte]

  implicit def wrapBytes(bytes: Array[Byte]): ByteBuffer = ByteBuffer.wrap(bytes)

  def apply(map: concurrent.Map[ByteBuffer, Value] = new ConcurrentHashMap[ByteBuffer, Value]().asScala): InMemoryKVStore = {
    new InMemoryKVStore(map)
  }

}
