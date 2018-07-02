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

package fluence.kad.grpc.client

import cats.effect.IO
import fluence.codec.PureCodec
import fluence.grpc.{GrpcHandler, ServiceManager}
import fluence.kad.{protobuf, protocol}
import fluence.kad.protocol.{Contact, KademliaRpc}
import io.grpc.{CallOptions, ManagedChannel}
import monix.execution.Scheduler

import scala.language.implicitConversions

object KademliaClientGrpc {

  /**
   * Shorthand to register KademliaClient inside NetworkClient.
   *
   * @param channelOptions     Channel to remote node and Call options
   */
  def register(serviceManager: ServiceManager)(
    channelOptions: IO[(ManagedChannel, CallOptions)]
  )(
    implicit
    codec: PureCodec[protocol.Node[Contact], protobuf.Node],
    ec: Scheduler
  ): KademliaRpc[Contact] = {
    val streamHandler = new GrpcHandler(serviceManager, channelOptions)
    new KademliaClient(streamHandler)
  }

}
