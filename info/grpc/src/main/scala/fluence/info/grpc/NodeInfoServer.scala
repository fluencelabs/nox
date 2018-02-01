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

package fluence.info.grpc

import cats.{ MonadError, ~> }
import cats.syntax.functor._
import cats.syntax.flatMap._
import fluence.info.NodeInfoRpc

import scala.concurrent.Future
import scala.language.higherKinds

class NodeInfoServer[F[_]](nodeInfo: NodeInfoRpc[F])(implicit F: MonadError[F, Throwable], run: F ~> Future)
  extends NodeInfoRpcGrpc.NodeInfoRpc {

  private val encode = NodeInfoCodec.codec[F].direct

  override def getInfo(request: NodeInfoRequest): Future[NodeInfo] =
    run(for {
      i ← nodeInfo.getInfo()
      res ← encode(i)
    } yield res)
}
