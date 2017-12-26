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

import cats.instances.try_._
import fluence.kad.protocol
import fluence.kad.protocol.{ Contact, Key, Node }
import fluence.transport.TransportServer
import fluence.transport.grpc.client.GrpcClientConf
import io.grpc._
import monix.eval.{ Coeval, Task }
import monix.execution.Scheduler.Implicits._

import scala.concurrent.Await
import scala.concurrent.duration.FiniteDuration
import scala.util.Try

/**
 * Server wrapper
 * @param server grpc Server instance
 * @param contact Self-discovered contact of this server
 * @param onShutdown Callback to launch on shutdown
 */
class GrpcServer private (
    server: Server,
    val contact: Task[Contact],
    onShutdown: Task[Unit]
) extends TransportServer {
  /**
   * Launch server, grab ports, or fail
   */
  def start(): Task[Unit] =
    Task(server.start())

  /**
   * Shut the server down, release ports
   */
  def shutdown(timeout: FiniteDuration): Unit = {
    server.shutdown()
    Await.ready(onShutdown.runAsync, timeout)
    server.awaitTermination()
  }

}

object GrpcServer {

  /**
   * Builder for grpc network server
   * @param contact Self-discovered contact for current node
   * @param shutdown What to call back on shutdown
   * @param localPort Local port to launch server on
   * @param services GRPC services definitions
   */
  class Builder(
      val contact: Task[Contact],
      shutdown: Task[Unit],
      localPort: Int,
      services: List[ServerServiceDefinition],
      interceptors: List[ServerInterceptor]) {
    /**
     * Add new grpc service to the server
     * @param service Service definition
     */
    def add(service: ServerServiceDefinition): Builder =
      new Builder(contact, shutdown, localPort, service :: services, interceptors)

    /**
     * Registers an interceptor.
     * Marked private, as it's easy to do weird things while writing interceptor.
     * But could be made public if necessary.
     *
     * @param interceptor Interceptor for incoming requests and messages
     */
    private def addInterceptor(interceptor: ServerInterceptor): Builder =
      new Builder(contact, shutdown, localPort, services, interceptor :: interceptors)

    private def readStringHeader(name: String, headers: Metadata): Option[String] =
      Option(headers.get(Metadata.Key.of(name, Metadata.ASCII_STRING_MARSHALLER)))

    /**
     * Register a callback to be called each time a node is active
     * @param cb To be called on ready and on each message
     * @param clientConf Conf to get header names from
     */
    def onNodeActivity(cb: Node[Contact] ⇒ Task[Any], clientConf: GrpcClientConf = GrpcClientConf.read()): Builder =
      addInterceptor(new ServerInterceptor {
        override def interceptCall[ReqT, RespT](call: ServerCall[ReqT, RespT], headers: Metadata, next: ServerCallHandler[ReqT, RespT]): ServerCall.Listener[ReqT] = {
          val remoteKey =
            readStringHeader(clientConf.keyHeader, headers).flatMap {
              b64key ⇒ Key.fromB64[Try](b64key).toOption
            }

          // TODO: check that contact IP matches request source, if it's possible
          val remoteContact =
            readStringHeader(clientConf.contactHeader, headers).flatMap {
              b64contact ⇒ Coeval.fromEval(Contact.readB64seed(b64contact)).attempt.value.toOption
            }

          def remoteNode: Option[Node[Contact]] =
            for {
              k ← remoteKey
              c ← remoteContact
            } yield protocol.Node(k, Instant.now(), c)

          def runCb(): Unit = remoteNode.map(cb).foreach(_.runAsync)

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
     * @return
     */
    def build: GrpcServer =
      new GrpcServer(
        {
          val sb = ServerBuilder
            .forPort(localPort)

          services.foreach(sb.addService)

          interceptors.reverseIterator.foreach(sb.intercept)

          sb.build()
        },
        contact,
        shutdown
      )
  }

  /**
   * Builder for a local port, no upnp
   * @param localPort Local port
   * @return
   */
  def builder(localPort: Int): Builder =
    new Builder(Task.now(Contact(InetAddress.getLocalHost, localPort)), Task.unit, localPort, Nil, Nil)

  /**
   * Builder for a local port, with upnp used to provide external port
   * @param localPort Local port
   * @param externalPort External port to be grabbed on gateway device
   * @param uPnP UPnP instance to use for gateway management
   * @return
   */
  def builder(localPort: Int, externalPort: Int, uPnP: UPnP = new UPnP()): Builder =
    new Builder(uPnP.addPort(externalPort, localPort).memoizeOnSuccess.onErrorRecover{
      case _ ⇒ Contact(InetAddress.getLocalHost, localPort)
    }, uPnP.deletePort(externalPort).memoizeOnSuccess, localPort, Nil, Nil)

  /**
   * Builder for config object
   * @param conf Server config object
   * @return
   */
  def builder(conf: GrpcServerConf): Builder =
    conf.externalPort.fold(builder(conf.localPort))(ext ⇒ builder(conf.localPort, ext))

  /**
   * Builder for default config object, read from typesafe conf
   * @return
   */
  def builder: Builder =
    builder(GrpcServerConf.read())
}
