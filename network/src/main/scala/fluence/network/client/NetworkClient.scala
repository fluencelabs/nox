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

package fluence.network.client

import java.util.concurrent.Executor

import fluence.kad.Key
import fluence.network.Contact
import io.grpc._
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import org.slf4j.LoggerFactory
import shapeless._

import scala.collection.concurrent.TrieMap

/**
 * Network Client caches managed channels to remote contacts, and service accesses to them
 *
 * @param buildStubs Build service stubs for a channel and call options
 * @param addHeaders Headers to be added to request. Keys must be valid ASCII string, see [[Metadata.Key]]
 * @tparam CL HList of all known services
 */
class NetworkClient[CL <: HList](
    buildStubs: (ManagedChannel, CallOptions) ⇒ CL,
    addHeaders: Task[Map[String, String]]) {
  private val log = LoggerFactory.getLogger(getClass)

  /**
   * Cache for available channels
   */
  private val channels = TrieMap.empty[String, ManagedChannel]

  /**
   * Cache for build service stubs
   */
  private val serviceStubs = TrieMap.empty[String, CL]

  /**
   * Convert Contact to a string key, to be used as a key in maps
   *
   * @param contact Contact
   */
  private def contactKey(contact: Contact): String =
    contact.b64seed

  /**
   * Returns cached or brand new ManagedChannel for a contact
   *
   * @param contact to open channel for
   */
  private def channel(contact: Contact): ManagedChannel =
    channels.getOrElseUpdate(
      contactKey(contact),
      {
        log.info("Open new channel: {}", contactKey(contact))
        ManagedChannelBuilder.forAddress(contact.ip.getHostAddress, contact.port)
          .usePlaintext(true)
          .build
      }
    )

  /**
   * Returns cached or brand new services HList for a contact
   *
   * @param contact to open channel and build service stubs for
   * @return HList of services for contact
   */
  private def services(contact: Contact): CL =
    serviceStubs.getOrElseUpdate(
      contactKey(contact),
      {
        log.info("Build services: {}", contactKey(contact))
        val ch = channel(contact)
        buildStubs(
          ch,
          CallOptions.DEFAULT.withCallCredentials( // TODO: is it a correct way to pass headers with the request?
            new CallCredentials {
              override def applyRequestMetadata(
                method: MethodDescriptor[_, _],
                attrs: Attributes,
                appExecutor: Executor,
                applier: CallCredentials.MetadataApplier): Unit = {

                val setHeaders = (headers: Map[String, String]) ⇒ {
                  log.trace("Writing metadata: {}", headers)
                  val md = new Metadata()
                  headers.foreach {
                    case (k, v) ⇒
                      md.put(Metadata.Key.of(k, Metadata.ASCII_STRING_MARSHALLER), v)
                  }
                  applier.apply(md)
                }

                // As addHeaders is memoized on success, this is effectively synchronous; see [[Task.runAsync]]
                addHeaders.attempt.runAsync.foreach {
                  case Right(headers) ⇒
                    setHeaders(headers)

                  case Left(err) ⇒
                    log.error("Cannot build network request headers!", err)
                    applier.fail(Status.UNKNOWN)
                }
              }

              override def thisUsesUnstableApi(): Unit = ()
            }))
      }
    )

  /**
   * Returns a service stub for a particular contact
   *
   * @param contact To open service for
   * @param sel     Implicit selector from HList
   * @tparam T Type of the service
   * @return
   */
  def service[T](contact: Contact)(implicit sel: ops.hlist.Selector[CL, T]): T =
    services(contact).select[T]
}

object NetworkClient {

  /**
   * Builder for NetworkClient
   *
   * @param buildStubs Builds all the known services for the channel and call ops
   * @param syncHeaders    Headers to pass with every request, known in advance
   * @param asyncHeaders Headers to pass with every request, not known in advance. Will be memoized on success
   * @tparam CL HList with all the services
   */
  class Builder[CL <: HList] private[NetworkClient] (
      buildStubs: (ManagedChannel, CallOptions) ⇒ CL,
      syncHeaders: Map[String, String],
      asyncHeaders: Task[Map[String, String]]) {
    self ⇒

    /**
     * Register a new service in the builder
     *
     * @param buildStub E.g. `new ServiceStub(_, _)`
     * @tparam T Type of the service
     */
    def add[T](buildStub: (ManagedChannel, CallOptions) ⇒ T): Builder[T :: CL] =
      new Builder[T :: CL]((ch, co) ⇒ buildStub(ch, co) :: self.buildStubs(ch, co), syncHeaders, asyncHeaders)

    /**
     * Add a header that will be passed with every request
     *
     * @param name  Header name, see [[Metadata.Key]] class comment for constraints
     * @param value Header value
     */
    def addHeader(name: String, value: String): Builder[CL] =
      new Builder(buildStubs, syncHeaders + (name -> value), asyncHeaders)

    /**
     * Adds a header asynchronously. Will be executed only once
     * @param header Header to add
     */
    def addAsyncHeader(header: Task[(String, String)]): Builder[CL] =
      new Builder(buildStubs, syncHeaders, for {
        hs ← asyncHeaders
        h ← header
      } yield hs + h)

    /**
     * Returns built NetworkClient
     *
     * @return
     */
    def build: NetworkClient[CL] = new NetworkClient[CL](buildStubs, asyncHeaders.map(_ ++ syncHeaders).memoizeOnSuccess)
  }

  /**
   * An empty builder
   */
  val builder: Builder[HNil] = new Builder[HNil]((_: ManagedChannel, _: CallOptions) ⇒ HNil, Map.empty, Task.now(Map.empty))

  /**
   * Builder with pre-defined credential headers
   * @param key This node's Kademlia key
   * @param contact This node's contact
   * @param conf Client config object
   * @return A NetworkClient builder
   */
  def builder(key: Key, contact: Task[Contact], conf: ClientConf = ClientConf.read()): Builder[HNil] =
    builder.addHeader(conf.keyHeader, key.b64).addAsyncHeader(contact.map(c ⇒ (conf.contactHeader, c.b64seed)))
}
