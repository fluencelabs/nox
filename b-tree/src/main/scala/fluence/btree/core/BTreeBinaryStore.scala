package fluence.btree.core

import cats.Functor
import fluence.btree.binary.Codec
import fluence.node.storage.KVStore
import monix.reactive.Observable

/**
 * Base implementation of [[BTreeStore]] for storing into binary KVStore.
 * @param kvStore key value store for persisting binary data
 * @param codec serializer for getting binary view for node and node id
 * @tparam Id type of node id
 * @tparam Node type of node
 * @tparam F - box for returning value
 */
class BTreeBinaryStore[Id, Node, F[_]: Functor](
    kvStore: KVStore[Array[Byte], Array[Byte], F, Observable],
    codec:   Codec[Any, Array[Byte]]
) extends BTreeStore[Id, Node, F] {

  override def get(key: Id): F[Node] = {
    Functor[F].map(kvStore.get(codec.encode(key)))(b ⇒ codec.decode[Node](b))
  }

  override def put(key: Id, node: Node): F[Unit] = {
    kvStore.put(codec.encode(key), codec.encode(node))
  }
}

object BTreeBinaryStore {
  def apply[Id, Node, F[_]: Functor](
    kvStore: KVStore[Array[Byte], Array[Byte], F, Observable],
    codec:   Codec[Any, Array[Byte]]
  ): BTreeBinaryStore[Id, Node, F] =
    new BTreeBinaryStore(kvStore, codec)
}
