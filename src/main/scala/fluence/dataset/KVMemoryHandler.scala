package fluence.dataset

import java.nio.ByteBuffer

import monix.eval.Task

import scala.collection.concurrent.TrieMap

object KVMemoryHandler {

  implicit object h extends KVStore.Handler[Task] {
    private val data =
      TrieMap.empty[ByteBuffer, Array[Byte]]

    override protected[this] def get(key: Array[Byte]): Task[Option[Array[Byte]]] =
      {
        println(s"get (${ByteBuffer.wrap(key).hashCode()}) from $data")
        Task.now(data.get(ByteBuffer.wrap(key)))
      }

    override protected[this] def put(key: Array[Byte], value: Array[Byte]): Task[Boolean] =
      {
        println(s"put (${ByteBuffer.wrap(key).hashCode()})=>[$value] to $data")
        Task.now {
          val r = data.put(ByteBuffer.wrap(key), value).isEmpty
          println(s"after put, data is $data")
          r
        }
      }

    override protected[this] def remove(key: Array[Byte]): Task[Boolean] =
      Task.now(data.remove(ByteBuffer.wrap(key)).isDefined)
  }

}
