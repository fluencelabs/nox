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

import cats.effect.IO
import fluence.transport.TransportServer
import io.grpc.ServerServiceDefinition
import io.grpc.inprocess.{InProcessChannelBuilder, InProcessServerBuilder}

/**
 * In process grpc server with lifecycle
 *
 */
class InProcessGrpcServer private (
  builderImpl: ⇒ IO[InProcessGrpcServer.Builder],
  onStartImpl: IO[Unit],
  onShutdownImpl: ⇒ IO[Unit]
) extends TransportServer[InProcessGrpcServer.Builder, InProcessGrpc] {
  override def onStart: IO[Unit] = onStartImpl

  override def onShutdown: IO[Unit] = onShutdownImpl

  override protected lazy val builder: IO[InProcessGrpcServer.Builder] = builderImpl

  override def startServer: InProcessGrpcServer.Builder ⇒ IO[InProcessGrpc] = _.build

  override def shutdownServer: InProcessGrpc ⇒ IO[Unit] = { s ⇒
    for {
      _ ← IO(s.server.shutdown())
      _ ← IO(s.server.awaitTermination())
      _ ← IO(s.channel.shutdown())
    } yield ()
  }

  def getServer: IO[InProcessGrpc] = {
    for {
      serverOp ← IO(serverRef.get)
      server ← serverOp match {
        case Some(s) ⇒ IO.pure(s)
        case None ⇒ IO.raiseError(new RuntimeException("Server was shutdown."))
      }
    } yield server
  }
}

object InProcessGrpcServer {
  case class Builder(
    name: String,
    services: List[ServerServiceDefinition]
  ) {

    def build: IO[InProcessGrpc] = {
      IO {
        val inProcessServer = InProcessServerBuilder.forName(name)
        services.foreach(s ⇒ inProcessServer.addService(s))
        val server = inProcessServer.build().start()

        val channel = InProcessChannelBuilder.forName(name).build()

        InProcessGrpc(server, channel)
      }
    }
  }

  def build(
    name: String,
    services: List[ServerServiceDefinition],
    onShutdown: IO[Unit] = IO.unit,
    onStart: IO[Unit] = IO.unit
  ): InProcessGrpcServer = {
    new InProcessGrpcServer(
      builderImpl = IO {
        Builder(name, services)
      },
      onStartImpl = onStart,
      onShutdownImpl = onShutdown
    )
  }
}
