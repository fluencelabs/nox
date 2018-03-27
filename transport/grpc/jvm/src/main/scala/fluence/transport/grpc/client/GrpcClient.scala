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

package fluence.transport.grpc.client

import java.util.concurrent.Executor

import cats.effect.IO
import fluence.kad.protocol.{Contact, Key}
import fluence.transport.TransportClient
import fluence.transport.grpc.GrpcConf
import io.grpc._
import shapeless._

import scala.collection.concurrent.TrieMap

/**
 * Network Client caches managed channels to remote contacts, and service accesses to them.
 *
 * @param buildStubs Build service stubs for a channel and call options
 * @param addHeaders Headers to be added to request. Keys must be valid ASCII string, see [[Metadata.Key]]
 * @tparam CL HList of all known services
 */
class GrpcClient[CL <: HList](
  buildStubs: (ManagedChannel, CallOptions) ⇒ CL,
  addHeaders: IO[Map[String, String]]
) extends TransportClient[Contact, CL] with slogging.LazyLogging {

  /**
   * Cache for available channels
   */
  private val channels = TrieMap.empty[String, ManagedChannel]

  /**
   * Cache for build service stubs
   */
  private val serviceStubs = TrieMap.empty[String, CL]

  /**
   * Convert Contact to a string key, to be used as a key in maps.
   *
   * @param contact Contact
   */
  private def contactKey(contact: Contact): String =
    contact.b64seed

  /**
   * Returns cached or brand new ManagedChannel for a contact.
   *
   * @param contact to open channel for
   */
  private def channel(contact: Contact): ManagedChannel =
    channels.getOrElseUpdate(
      contactKey(contact), {
        logger.debug("Open new channel: {}", contactKey(contact))
        ManagedChannelBuilder
          .forAddress(contact.addr, contact.grpcPort)
          .usePlaintext(true)
          .build
      }
    )

  /**
   * Returns cached or brand new services HList for a contact.
   *
   * @param contact to open channel and build service stubs for
   * @return HList of services for contact
   */
  private def services(contact: Contact): CL =
    serviceStubs.getOrElseUpdate(
      contactKey(contact), {
        logger.info("Build services: {}", contactKey(contact))
        val ch = channel(contact)
        buildStubs(
          ch,
          CallOptions.DEFAULT.withCallCredentials( // TODO: is it a correct way to pass headers with the request?
            new CallCredentials {
              override def applyRequestMetadata(
                method: MethodDescriptor[_, _],
                attrs: Attributes,
                appExecutor: Executor,
                applier: CallCredentials.MetadataApplier
              ): Unit = {

                val setHeaders = (headers: Map[String, String]) ⇒ {
                  logger.trace("Writing metadata: {}", headers)
                  val md = new Metadata()
                  headers.foreach {
                    case (k, v) ⇒
                      md.put(Metadata.Key.of(k, Metadata.ASCII_STRING_MARSHALLER), v)
                  }
                  applier.apply(md)
                }

                addHeaders.unsafeRunAsync {
                  case Right(headers) ⇒
                    setHeaders(headers)

                  case Left(err) ⇒
                    logger.error("Cannot build network request headers!", err)
                    applier.fail(Status.UNKNOWN)
                }
              }

              override def thisUsesUnstableApi(): Unit = ()
            }
          )
        )
      }
    )

  /**
   * Returns a service stub for a particular contact.
   *
   * @param contact To open service for
   * @param sel     Implicit selector from HList
   * @tparam T Type of the service
   * @return
   */
  override def service[T](contact: Contact)(implicit sel: ops.hlist.Selector[CL, T]): T = {
    logger.trace(s"Going to retrieve service for contact $contact")
    services(contact).select[T]
  }
}

object GrpcClient {

  /**
   * Builder for NetworkClient.
   *
   * @param buildStubs   Builds all the known services for the channel and call ops
   * @param syncHeaders  Headers to pass with every request, known in advance
   * @param asyncHeaders Headers to pass with every request, not known in advance. Will be memoized on success
   * @tparam CL HList with all the services
   */
  class Builder[CL <: HList] private[GrpcClient] (
    buildStubs: (ManagedChannel, CallOptions) ⇒ CL,
    syncHeaders: Map[String, String],
    asyncHeaders: IO[Map[String, String]]
  ) {
    self ⇒

    /**
     * Register a new service in the builder.
     *
     * @param buildStub E.g. `new ServiceStub(_, _)`
     * @tparam T Type of the service
     */
    def add[T](buildStub: (ManagedChannel, CallOptions) ⇒ T): Builder[T :: CL] =
      new Builder[T :: CL]((ch, co) ⇒ buildStub(ch, co) :: self.buildStubs(ch, co), syncHeaders, asyncHeaders)

    /**
     * Add a header that will be passed with every request.
     *
     * @param name  Header name, see [[Metadata.Key]] class comment for constraints
     * @param value Header value
     */
    def addHeader(name: String, value: String): Builder[CL] =
      new Builder(buildStubs, syncHeaders + (name -> value), asyncHeaders)

    /**
     * Adds a header asynchronously. Will be executed only once.
     *
     * @param name  Header to add
     * @param value Header value getter
     */
    def addHeaderIO(name: String, value: IO[String]): Builder[CL] =
      new Builder(buildStubs, syncHeaders, for {
        hs ← asyncHeaders
        v ← value
      } yield hs + (name -> v))

    /**
     * Returns built NetworkClient.
     *
     * @return
     */
    def build: GrpcClient[CL] = new GrpcClient[CL](buildStubs, asyncHeaders.map(_ ++ syncHeaders))
  }

  /**
   * An empty builder.
   */
  val builder: Builder[HNil] =
    new Builder[HNil]((_: ManagedChannel, _: CallOptions) ⇒ HNil, Map.empty, IO.pure(Map.empty))

  /**
   * Builder with pre-defined credential headers.
   *
   * @param key         This node's Kademlia key
   * @param contactSeed This node's contact, serialized. Notice that you must memoize value yourself, if it should be memoized
   * @param conf        Client config object
   * @return A NetworkClient builder
   */
  def builder(key: Key, contactSeed: IO[String], conf: GrpcConf): Builder[HNil] =
    builder
      .addHeader(conf.keyHeader, key.b64)
      .addHeaderIO(conf.contactHeader, contactSeed)
}
