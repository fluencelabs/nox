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

import cats.data.Kleisli
import cats.effect.IO
import fluence.transport.TransportServer
import io.grpc._

class GrpcServer private (
  builderImpl: ⇒ IO[Server],
  onStartImpl: IO[Unit],
  onShutdownImpl: ⇒ IO[Unit]
) extends TransportServer[Server, Server] {

  override def startServer: Server ⇒ IO[Server] = s ⇒ IO(s.start())

  override def shutdownServer: Server ⇒ IO[Unit] = { s ⇒
    for {
      _ ← IO(s.shutdown())
      _ ← IO(s.awaitTermination())
    } yield ()
  }

  override def onStart: IO[Unit] = onStartImpl

  override def onShutdown: IO[Unit] = onShutdownImpl

  override lazy val builder: IO[Server] = builderImpl
}

object GrpcServer extends slogging.LazyLogging {

  /**
   *
   * @param onShutdown   Callback to be launched before server shut down
   * @param onStart      Callback to be launched before server starts
   * @param port         Most accessible port
   * @param services     List of services to register with the server
   * @param interceptors List of call interceptors to register with the server
   */
  case class Builder(
    onShutdown: IO[Unit],
    onStart: IO[Unit],
    port: Int,
    services: List[ServerServiceDefinition],
    interceptors: List[ServerInterceptor]
  ) {

    /**
     * Add new grpc service to the server
     *
     * @param service Service definition
     */
    def add(service: ServerServiceDefinition): Builder =
      copy(services = service :: services)

    /**
     * Registers an interceptor.
     * Marked private, as it's easy to do weird things while writing interceptor.
     * But could be made public if necessary.
     *
     * @param interceptor Interceptor for incoming requests and messages
     */
    private def addInterceptor(interceptor: ServerInterceptor): Builder =
      copy(interceptors = interceptor :: interceptors)

    private def readStringHeader(name: String, headers: Metadata): Option[String] =
      Option(headers.get(Metadata.Key.of(name, Metadata.ASCII_STRING_MARSHALLER)))

    /**
     * Register a callback to be called each time a node is active
     *
     * @param cb         To be called on ready and on each message, gets headers and optional message as an input
     */
    def onCall(cb: (Kleisli[Option, String, String], Option[Any]) ⇒ IO[Unit]): Builder =
      addInterceptor(new ServerInterceptor {
        override def interceptCall[ReqT, RespT](
          call: ServerCall[ReqT, RespT],
          headers: Metadata,
          next: ServerCallHandler[ReqT, RespT]
        ): ServerCall.Listener[ReqT] = {

          val headersK = Kleisli[Option, String, String](readStringHeader(_, headers))

          val listener = next.startCall(call, headers)

          new ForwardingServerCallListener.SimpleForwardingServerCallListener[ReqT](listener) {

            override def onMessage(message: ReqT): Unit = {
              cb(headersK, Some(message)).unsafeRunSync()
              super.onMessage(message)
            }

            override def onReady(): Unit = {
              cb(headersK, None).unsafeRunSync()
              super.onReady()
            }
          }
        }
      })

    /**
     * Build grpc server with all the defined services
     *
     * @return
     */
    def build: GrpcServer = {
      new GrpcServer(
        builderImpl = IO {
          logger.info(s"Building GRPC server forPort($port)")

          val sb = ServerBuilder
            .forPort(port)

          services.foreach(sb.addService)

          interceptors.reverseIterator.foreach(sb.intercept)

          sb.build()
        },
        onStartImpl = onStart,
        onShutdownImpl = onShutdown
      )
    }
  }

  /**
   * Builder for config object
   *
   * @param conf Server config object
   */
  def builder(conf: GrpcServerConf): Builder = {
    val port = conf.port
    Builder(
      onStart = IO(logger.info("Server started on port " + port)),
      onShutdown = IO(logger.info("Shut down on port: " + port)),
      port = conf.port,
      services = Nil,
      interceptors = Nil
    )
  }
}
