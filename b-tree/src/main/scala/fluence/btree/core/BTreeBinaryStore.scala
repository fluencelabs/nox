package fluence.btree.core

import cats.MonadError
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.btree.binary.Codec
import fluence.node.storage.KVStore
import monix.reactive.Observable

import scala.language.higherKinds

/**
 * Base implementation of [[BTreeStore]] for storing into binary KVStore.
 *
 * @param kvStore   key value store for persisting binary data
 * @param idCodec   serializer for getting binary view for node id
 * @param nodeCodec serializer for getting binary view for node
 * @tparam Id   type of node id
 * @tparam Node type of node
 */
class BTreeBinaryStore[Id, Node, F[_]](
    kvStore: KVStore[Array[Byte], Array[Byte], F, Observable]
)(
    implicit
    idCodec: Codec[Id, Array[Byte], F],
    nodeCodec: Codec[Node, Array[Byte], F],
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