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

package fluence.grpc.proxy

import java.util.concurrent.TimeUnit

import cats.effect.IO
import io.grpc.inprocess.{InProcessChannelBuilder, InProcessServerBuilder}
import io.grpc._

import scala.collection.JavaConverters._
import scala.language.higherKinds

/**
 * Grpc server and channel, that work in memory without transport.
 */
final class InProcessGrpc private (private val server: Server, private val channel: ManagedChannel) {
  val services: List[ServerServiceDefinition] = server.getServices.asScala.toList

  /**
   * Create new call from channel to server.
   * @param methodDescriptor The method descrpiptor to be used for the call.
   * @param callOptions The collection of runtime options for a new RPC call.
   */
  def newCall[Req, Resp](
    methodDescriptor: MethodDescriptor[Req, Resp],
    callOptions: CallOptions
  ): ClientCall[Req, Resp] = {
    channel.newCall[Req, Resp](methodDescriptor, callOptions)
  }

  /**
   * Once the server and the channel are closed, it will throw errors on each call.
   */
  def close(): IO[Unit] = {
    for {
      _ ← IO(server.shutdown())
      _ ← IO(server.awaitTermination())
      _ ← IO(channel.shutdown())
      _ ← IO(channel.awaitTermination(10L, TimeUnit.SECONDS))
    } yield ()
  }
}

object InProcessGrpc {

  def build(name: String, services: List[ServerServiceDefinition]): IO[InProcessGrpc] = {
    for {
      server ← IO {
        val builder = InProcessServerBuilder.forName(name)
        services.foreach(s ⇒ builder.addService(s))
        builder.build().start()
      }
      channel ← IO(InProcessChannelBuilder.forName(name).build())
    } yield new InProcessGrpc(server, channel)
  }
}
