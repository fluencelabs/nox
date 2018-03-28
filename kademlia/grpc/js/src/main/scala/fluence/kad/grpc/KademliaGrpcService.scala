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

package fluence.kad.grpc

import fluence.kad.grpc.facade._

import scala.concurrent.Future

trait KademliaGrpcService {
  def ping(request: PingRequest): scala.concurrent.Future[Node]
  def lookup(request: LookupRequest): scala.concurrent.Future[NodesResponse]
  def lookupAway(request: LookupAwayRequest): scala.concurrent.Future[NodesResponse]
}

object KademliaGrpcService {

  def apply(host: String, debug: Boolean = false): KademliaGrpcService =
    new KademliaGrpcService() with slogging.LazyLogging {
      private val grpc = new GrpcJSService(host, debug)

      override def ping(request: PingRequest): Future[Node] = {
        grpc.unary(KademliaDescriptors.ping, request)
      }

      override def lookup(request: LookupRequest): Future[NodesResponse] = {
        grpc.unary(KademliaDescriptors.lookup, request)
      }

      override def lookupAway(request: LookupAwayRequest): Future[NodesResponse] = {
        grpc.unary(KademliaDescriptors.lookupAway, request)
      }
    }
}
