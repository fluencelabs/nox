package fluence.btree.core

import fluence.btree.binary.Codec
import fluence.node.storage.KVStore
import monix.eval.{ Coeval, Task }
import monix.reactive.Observable

/**
 * Base implementation of [[BTreeStore]] for storing into binary KVStore.
 * @param kvStore key value store for persisting binary data
 * @param idCodec serializer for getting binary view for node id
 * @param nodeCodec serializer for getting binary view for node
 * @tparam Id type of node id
 * @tparam Node type of node
 */
class BTreeBinaryStore[Id, Node](
    kvStore:   KVStore[Array[Byte], Array[Byte], Task, Observable],
    idCodec:   Codec[Id, Array[Byte], Option],
    nodeCodec: Codec[Node, Array[Byte], Option]
) extends BTreeStore[Id, Node, Task] {

  override def get(id: Id): Task[Node] = {
    idCodec.encode(id)
      .map(binId ⇒ {
        kvStore.get(binId)
          .flatMap(binNode ⇒ {
            nodeCodec.decode(binNode)
              .map(Task.now)
              .getOrElse(Task.raiseError(new IllegalArgumentException(s"Cant get node from binary view for binNode = $binNode")))
          })
      })
      .getOrElse(Task.raiseError(new IllegalArgumentException(s"Cant get binary view for id = $id")))
  }

  override def put(id: Id, node: Node): Task[Unit] = {
    val result = for {
      binId ← idCodec.encode(id)
      binNode ← nodeCodec.encode(node)
    } yield kvStore.put(binId, binNode)
    result.getOrElse(Task.raiseError(new IllegalArgumentException(s"Cant get binary view for id = $id or node = $node")))
  }

}

object BTreeBinaryStore {
  def apply[Id, Node](
    kvStore:   KVStore[Array[Byte], Array[Byte], Task, Observable],
    idCodec:   Codec[Id, Array[Byte], Option],
    nodeCodec: Codec[Node, Array[Byte], Option]
  ): BTreeBinaryStore[Id, Node] =
    new BTreeBinaryStore(kvStore, idCodec, nodeCodec)
}
