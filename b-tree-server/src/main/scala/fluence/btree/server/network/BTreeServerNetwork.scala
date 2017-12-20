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

package fluence.btree.server.network

import fluence.btree.client.network.{ BTreeServerResponse, InitClientRequest, _ }
import fluence.btree.server.network.BTreeServerNetwork.RequestHandler

/**
 * Base network abstraction for interaction BTree server with the BTree client.
 *
 * @tparam F The type of effect, box for returning value
 */
trait BTreeServerNetwork[F[_]] {

  /**
   * Handler for init request [[fluence.btree.client.network.InitClientRequest]]
   * from [[fluence.btree.client.MerkleBTreeClient]] contains all business logic
   * for processing client init requests.
   */
  val initRequestHandler: RequestHandler[F]

  /** Start the BTreeServerNetwork. */
  def start(): F[Boolean]

  /** Shut the BTreeServerNetwork down. */
  def shutdown(): F[Boolean]

  // todo implement with transport, this is pseudocode for example!

  //  def get(requests: Observable[BTreeClientRequest]): Observable[BTreeServerResponse] = {
  //    requests.getNext().flatMap {
  //      case initReq: InitClientRequest =>
  //        val result = Observable.new[BTreeServerResponse]
  //
  //      val askClientFn: BTreeServerResponse ⇒ F[BTreeClientRequest] =
  //        response ⇒ {
  //          result.push(GRPC.CmpCommand(response)).flatMap { _ =>
  //            requests.getNext().as[F[BTreeClientRequest]]
  //          }
  //        }
  //
  //      initRequestHandler(initReq, askClientFn)
  //    }
  //    ...
  //  }

}

object BTreeServerNetwork {

  type RequestHandler[F[_]] = (InitClientRequest, BTreeServerResponse ⇒ F[Option[BTreeClientRequest]]) ⇒ F[Unit]

}
