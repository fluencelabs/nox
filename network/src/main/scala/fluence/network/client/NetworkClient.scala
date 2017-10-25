package fluence.network.client

import fluence.network.Contact
import io.grpc.{ CallOptions, ManagedChannel, ManagedChannelBuilder }
import org.slf4j.LoggerFactory
import shapeless._

import scala.collection.concurrent.TrieMap

/**
 * Network Client caches managed channels to remote contacts, and service accesses to them
 * @param buildStubs Build service stubs for a channel and call options
 * @tparam CL HList of all known services
 */
class NetworkClient[CL <: HList](buildStubs: (ManagedChannel, CallOptions) ⇒ CL) {
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
   * @param contact Contact
   * @return
   */
  private def contactKey(contact: Contact): String =
    contact.ip.getHostAddress + ":" + contact.port

  /**
   * Returns cached or brand new ManagedChannel for a contact
   * @param contact to open channel for
   * @return
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
   * @param contact to open channel and build service stubs for
   * @return
   */
  private def services(contact: Contact): CL =
    serviceStubs.getOrElseUpdate(
      contactKey(contact),
      {
        log.info("Build services: {}", contactKey(contact))
        val ch = channel(contact)
        buildStubs(ch, CallOptions.DEFAULT)
      }
    )

  /**
   * Returns a service stub for a particular contact
   * @param contact To open service for
   * @param sel Implicit selector from HList
   * @tparam T Type of the service
   * @return
   */
  def service[T](contact: Contact)(implicit sel: ops.hlist.Selector[CL, T]): T =
    services(contact).select[T]
}

object NetworkClient {

  /**
   * Builder for NetworkClient
   * @tparam CL HList with all the services
   */
  abstract class Builder[CL <: HList] {
    self ⇒
    /**
     * Builds all the known services for the channel and call ops
     * @return
     */
    def buildStubs: (ManagedChannel, CallOptions) ⇒ CL

    /**
     * Register a new service in the builder
     * @param buildStub E.g. `new ServiceStub(_, _)`
     * @tparam T Type of the service
     * @return
     */
    def add[T](buildStub: (ManagedChannel, CallOptions) ⇒ T): Builder[T :: CL] =
      new Builder[T :: CL] {
        override def buildStubs: (ManagedChannel, CallOptions) ⇒ T :: CL =
          (ch, co) ⇒ buildStub(ch, co) :: self.buildStubs(ch, co)
      }

    /**
     * Returns built NetworkClient
     * @return
     */
    def build: NetworkClient[CL] = new NetworkClient[CL](buildStubs)
  }

  /**
   * An empty builder
   */
  val builder: Builder[HNil] = new Builder[HNil] {
    override def buildStubs: (ManagedChannel, CallOptions) ⇒ HNil =
      (_, _) ⇒ HNil
  }
}