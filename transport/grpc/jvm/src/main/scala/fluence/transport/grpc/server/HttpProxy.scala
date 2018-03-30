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

package fluence.transport.grpc.server

import cats.effect.IO
import fluence.transport.TransportServer
import io.netty.bootstrap.ServerBootstrap
import io.netty.channel.nio.NioEventLoopGroup
import io.netty.channel.socket.nio.NioServerSocketChannel
import io.netty.channel.{ChannelFuture, ChannelOption}
import io.netty.handler.logging.{LogLevel, LoggingHandler}

class HttpProxy private (
  port: Int,
  builderImpl: ⇒ IO[ServerBootstrap],
  onStartImpl: IO[Unit],
  onShutdownImpl: ⇒ IO[Unit]
) extends TransportServer[ServerBootstrap, ChannelFuture] {

  override def startServer(): ServerBootstrap ⇒ IO[ChannelFuture] = { sb ⇒
    IO(sb.bind(port).sync())
  }

  override val onStart: IO[Unit] = onStartImpl

  override lazy val onShutdown: IO[Unit] = onShutdownImpl

  override lazy val builder: IO[ServerBootstrap] = builderImpl

  override def shutdownServer: ChannelFuture ⇒ IO[Unit] = { cf ⇒
    IO(cf.channel().closeFuture().sync())
  }
}

object HttpProxy extends slogging.LazyLogging {

  def apply(port: Int): HttpProxy = {

    val bootstrap = IO {
      logger.info(s"Building GRPC server forPort($port)")

      val bossGroup = new NioEventLoopGroup(1)
      val workerGroup = new NioEventLoopGroup()
      val b = new ServerBootstrap()
      b.group(bossGroup, workerGroup)
        .channel(classOf[NioServerSocketChannel])
        .handler(new LoggingHandler(LogLevel.INFO))
        .childOption(ChannelOption.AUTO_READ.asInstanceOf[ChannelOption[Boolean]], false)
        .childHandler(new HttpProxyHandler())

    }

    new HttpProxy(
      port = port,
      builderImpl = bootstrap,
      onStartImpl = IO(logger.info("Http server started on port " + port)),
      onShutdownImpl = IO(logger.info("Http server down on port: " + port))
    )
  }
}
