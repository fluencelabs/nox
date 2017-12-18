package fluence.btree.server.binary

import cats.MonadError
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.btree.server.core.BTreeStore
import fluence.node.binary.Codec
import fluence.node.storage.KVStore

import scala.language.higherKinds

/**
 * Base implementation of [[BTreeStore]] that stores tree nodes in the binary key-value store.
 *
 * @param kvStore   Binary key-value store
 * @param idCodec   Node id binary serializer
 * @param nodeCodec Node binary serializer
 * @tparam Id   The type of node id
 * @tparam Node The type of node
 * @tparam F - Box for returning value
 */
class BTreeBinaryStore[Id, Node, F[_]](
    kvStore: KVStore[F, Array[Byte], Array[Byte]]
)(
    implicit
    idCodec: Codec[F, Id, Array[Byte]],
    nodeCodec: Codec[F, Node, Array[Byte]],
    F: MonadError[F, Throwable]
) extends BTreeStore[Id, Node, F] {

  override def get(id: Id): F[Node] =
    for {
      binId ← idCodec.encode(id)
      binNode ← kvStore.get(binId)
      node ← nodeCodec.decode(binNode)
    } yield node

  override def put(id: Id, node: Node): F[Unit] =
    for {
      binId ← idCodec.encode(id)
      binNode ← nodeCodec.encode(node)
      _ ← kvStore.put(binId, binNode)
    } yield ()

}
