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

import java.net.InetAddress
import java.time.Instant
import java.util.concurrent.atomic.AtomicReference

import cats._
import cats.effect.IO
import cats.instances.try_._
import fluence.crypto.signature.SignatureChecker
import fluence.kad.protocol
import fluence.kad.protocol.{Contact, Key, Node}
import fluence.transport.TransportServer
import fluence.transport.grpc.GrpcConf
import io.grpc._

import scala.util.Try

/**
  * Server wrapper
  *
  * @param server     grpc Server instance
  * @param address    Most accessible address for the server (may be local)
  * @param port       Most accessible port for the server (may be local)
  * @param onStart    Callback to launch before start; should be used to grab UPnP ports etc.
  * @param onShutdown Callback to launch on shutdown; should be used to release UPnP ports etc.
  */
class GrpcServer private (
  server: ⇒ Server,
  val address: InetAddress,
  val port: Int,
  onStart: IO[Unit],
  onShutdown: IO[Unit]
) extends TransportServer with slogging.LazyLogging {
  private val serverRef = new AtomicReference[Server](null)

  /**
    * Launch server, grab ports, or fail
    */
  val start: IO[Unit] =
    for {
      _ ← if (serverRef.get() == null) IO.unit else shutdown
      _ ← onStart
      s ← IO(server.start())
    } yield {
      serverRef.set(s)
      logger.info("Server started on port " + port)
    }

  /**
    * Shut the server down, release ports
    */
  lazy val shutdown: IO[Unit] =
    Option(serverRef.getAndSet(null)).fold(IO(logger.debug("Already shut down? " + port)))(srv ⇒
      for {
        _ ← IO(srv.shutdown())
        _ ← onShutdown
        _ ← IO(srv.awaitTermination())
      } yield logger.info("Shut down on port: " + port))

}

object GrpcServer extends slogging.LazyLogging {

  /**
    *
    * @param onShutdown   Callback to be launched before server shut down
    * @param onStart      Callback to be launched before server starts
    * @param address      Most accessible address
    * @param port         Most accessible port
    * @param services     List of services to register with the server
    * @param interceptors List of call interceptors to register with the server
    */
  case class Builder(
    onShutdown: IO[Unit],
    onStart: IO[Unit],
    address: InetAddress,
    port: Int,
    services: List[ServerServiceDefinition],
    interceptors: List[ServerInterceptor]) {

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
      * @param cb         To be called on ready and on each message
      * @param clientConf Conf to get header names from
      */
    def onNodeActivity(cb: Node[Contact] ⇒ IO[Any], clientConf: GrpcConf)(implicit checker: SignatureChecker): Builder =
      addInterceptor(new ServerInterceptor {
        override def interceptCall[ReqT, RespT](
          call: ServerCall[ReqT, RespT],
          headers: Metadata,
          next: ServerCallHandler[ReqT, RespT]): ServerCall.Listener[ReqT] = {
          val remoteKey =
            readStringHeader(clientConf.keyHeader, headers).flatMap { b64key ⇒
              Key.fromB64[Try](b64key).toOption
            }

          // TODO: check that contact IP matches request source, if it's possible
          val remoteContact =
            readStringHeader(clientConf.contactHeader, headers).flatMap { b64contact ⇒
              Contact.readB64seed[Id](b64contact).value.toOption
            }

          def remoteNode: Option[Node[Contact]] =
            for {
              k ← remoteKey
              c ← remoteContact
            } yield protocol.Node(k, Instant.now(), c)

          def runCb(): Unit = remoteNode.map(cb).foreach(_.unsafeRunAsync(_ ⇒ ()))

          val listener = next.startCall(call, headers)

          new ForwardingServerCallListener.SimpleForwardingServerCallListener[ReqT](listener) {

            override def onMessage(message: ReqT): Unit = {
              runCb()
              super.onMessage(message)
            }

            override def onReady(): Unit = {
              runCb()
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
    def build: GrpcServer =
      new GrpcServer(
        server = {
          logger.info(s"Building GRPC server forPort($port)")

          val sb = ServerBuilder
            .forPort(port)

          services.foreach(sb.addService)

          interceptors.reverseIterator.foreach(sb.intercept)

          sb.build()
        },
        address = address,
        port = port,
        onStart = onStart,
        onShutdown = onShutdown
      )
  }

  /**
    * Builder for config object
    *
    * @param conf Server config object
    */
  def builder(conf: GrpcServerConf): Builder =
    Builder(
      onStart = IO.unit,
      onShutdown = IO.unit,
      address = InetAddress.getLocalHost,
      port = conf.port,
      services = Nil,
      interceptors = Nil
    )
}
