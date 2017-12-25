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
 * @tparam F Box for returning value
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
