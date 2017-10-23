package fluence.network.client

import java.net.InetAddress
import java.time.Instant

import com.google.protobuf.ByteString
import fluence.kad.{ KademliaRPC, Key, Node }
import fluence.network.{ Contact, proto }
import fluence.network.proto.kademlia.{ Header, KademliaGrpc, LookupRequest, PingRequest }
import io.grpc.{ CallOptions, ManagedChannel }
import monix.eval.Task
import org.slf4j.LoggerFactory
import shapeless._

import scala.language.implicitConversions

/**
 * Implementation of KademliaClient over GRPC, with Task and Contact
 * @param header Header to pass with all requests
 * @param stub GRPC Kademlia Stub
 */
class KademliaClient(header: Task[Option[Header]], stub: KademliaGrpc.KademliaStub) extends KademliaRPC[Task, Contact] {

  private val log = LoggerFactory.getLogger(getClass)

  private implicit def nToNc(n: proto.kademlia.Node): Node[Contact] = Node[Contact](
    Key(n.id.toByteArray),
    Instant.now(),
    Contact(
      InetAddress.getByAddress(n.ip.toByteArray),
      n.port
    )
  )

  /**
   * Ping the contact, get its actual Node status, or fail
   *
   * @return
   */
  override def ping(): Task[Node[Contact]] =
    for {
      h ← header
      _ = log.debug("Querying ping to: {}", h.map(_.to))
      n ← Task.deferFuture(stub.ping(PingRequest(h)))
    } yield n: Node[Contact]

  /**
   * Perform a local lookup for a key, return K closest known nodes
   *
   * @param key Key to lookup
   * @return
   */
  override def lookup(key: Key, numberOfNodes: Int): Task[Seq[Node[Contact]]] =
    for {
      h ← header
      _ = log.debug(s"Calling lookup($key, $numberOfNodes) to: {}", h.map(_.to))
      res ← Task.deferFuture(stub.lookup(LookupRequest(h, ByteString.copyFrom(key.id), numberOfNodes)))
    } yield res.nodes.map(n ⇒ n: Node[Contact])

  /**
   * Perform an iterative lookup for a key, return K closest known nodes
   *
   * @param key Key to lookup
   * @return
   */
  override def lookupIterative(key: Key, numberOfNodes: Int): Task[Seq[Node[Contact]]] =
    for {
      h ← header
      _ = log.debug(s"Calling lookup iterative($key, $numberOfNodes) to: {}", h.map(_.to))
      res ← Task.deferFuture(stub.lookupIterative(LookupRequest(h, ByteString.copyFrom(key.id), numberOfNodes)))
    } yield res.nodes.map(n ⇒ n: Node[Contact])

}

object KademliaClient {
  /**
   * Shorthand to register KademliaClient inside NetworkClient
   * @param header Header to pass with all requests
   * @param channel Channel to remote node
   * @param callOptions Call options
   * @return
   */
  def register(header: Task[Option[Header]])(channel: ManagedChannel, callOptions: CallOptions): KademliaClient =
    new KademliaClient(header, new KademliaGrpc.KademliaStub(channel, callOptions))

  /**
   * Kademlia client summoner for NetworkClient
   * @param client Network client
   * @param sel HList selector
   * @tparam CL HList of network services
   * @return
   */
  def apply[CL <: HList](client: NetworkClient[CL])(implicit sel: ops.hlist.Selector[CL, KademliaClient]): Contact ⇒ KademliaClient =
    client.service[KademliaClient](_)(sel)
}