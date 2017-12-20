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

package fluence.network.server

import com.google.protobuf.ByteString
import fluence.kad.{ Kademlia, Key }
import fluence.network.Contact
import fluence.network.proto.kademlia._
import monix.eval.Task
import monix.execution.Scheduler
import org.slf4j.LoggerFactory

import scala.concurrent.Future
import scala.language.implicitConversions

// TODO: cover with tests
// TODO: deduplicate Task[T] => Future[T] running, with automatic logging
class KademliaServerImpl(kad: Kademlia[Task, Contact])(implicit sc: Scheduler) extends KademliaGrpc.Kademlia {
  private val log = LoggerFactory.getLogger(getClass)

  private implicit def ncToNode(nc: fluence.kad.Node[Contact]): Node =
    Node(id = ByteString.copyFrom(nc.key.id), ByteString.copyFrom(nc.contact.ip.getAddress), nc.contact.port)

  override def ping(request: PingRequest): Future[Node] = {
    log.debug(s"${kad.nodeId} / Incoming ping")

    kad.handleRPC
      .ping()
      .map(nc ⇒ nc: Node)
      .map{ n ⇒ log.debug(s"Ping reply: {}", n); n }

      .onErrorRecoverWith{
        case e ⇒
          log.warn("Can't reply on ping!", e)
          Task.raiseError(e)
      }.runAsync
  }

  override def lookup(request: LookupRequest): Future[NodesResponse] = {
    log.debug(s"${kad.nodeId} / Incoming lookup: for {}", Key(request.key.toByteArray))

    kad.handleRPC
      .lookup(Key(request.key.toByteArray), request.numberOfNodes)
      .map{ n ⇒ log.debug(s"Lookup reply: {}", n); n }
      .map(_.map(nc ⇒ nc: Node))
      .map(NodesResponse(_))

      .onErrorRecoverWith{
        case e ⇒
          log.warn("Can't reply on lookup!", e)
          Task.raiseError(e)
      }.runAsync
  }

  override def lookupAway(request: LookupAwayRequest): Future[NodesResponse] = {
    log.debug(s"${kad.nodeId} / Incoming lookupAway: for {}", Key(request.key.toByteArray))

    kad.handleRPC
      .lookupAway(Key(request.key.toByteArray), Key(request.moveAwayFrom.toByteArray), request.numberOfNodes)
      .map{ n ⇒ log.debug(s"LookupAway reply: {}", n); n }
      .map(_.map(nc ⇒ nc: Node))
      .map(NodesResponse(_))

      .onErrorRecoverWith{
        case e ⇒
          log.warn("Can't reply on lookup!", e)
          Task.raiseError(e)
      }.runAsync
  }

  override def lookupIterative(request: LookupRequest): Future[NodesResponse] = {
    log.debug(s"${kad.nodeId} / Incoming lookup iterative: for {}", Key(request.key.toByteArray))

    kad.handleRPC
      .lookupIterative(Key(request.key.toByteArray), request.numberOfNodes)
      .map{ n ⇒ log.debug(s"Reply to lookupIterative: $n"); n }
      .map(_.map(nc ⇒ nc: Node))
      .map(NodesResponse(_))

      .onErrorRecoverWith{
        case e ⇒
          log.warn("can't reply on lookup iterative!", e)
          Task.raiseError(e)
      }.runAsync
  }
}
