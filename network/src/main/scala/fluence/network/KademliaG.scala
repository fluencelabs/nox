package fluence.network

import java.net.InetAddress
import java.time.Instant

import cats.Id
import cats.data.StateT
import cats.syntax.show._
import com.google.protobuf.ByteString
import fluence.kad._
import fluence.network.proto.kademlia.{ Header, KademliaGrpc, LookupRequest, PingRequest }
import io.grpc.{ ManagedChannel, ManagedChannelBuilder }
import monix.eval.{ MVar, Task, TaskSemaphore }
import monix.execution.atomic.{ AtomicAny, AtomicBoolean }
import org.slf4j.LoggerFactory

import scala.collection.concurrent.TrieMap
import scala.concurrent.duration._
import scala.language.implicitConversions

class KademliaG(val key: Key, contact: ⇒ Contact) extends Kademlia[Task, Contact](Alpha = 3, K = 2, pingTimeout = 1.second) {
  private val keyByteString = ByteString.copyFrom(key.id)

  private val readState = AtomicAny(RoutingTable[Contact](key, K, K))

  private def joined = readState.get.initialized

  private val state = MVar(readState.get)
  private val semaphore = TaskSemaphore(1)

  private val channels = TrieMap.empty[String, ManagedChannel]

  private val log = LoggerFactory.getLogger(getClass)

  private implicit def nToNc(n: proto.kademlia.Node): Node[Contact] = Node[Contact](
    Key(n.id.toByteArray),
    Instant.now(),
    Contact(
      InetAddress.getByAddress(n.ip.toByteArray),
      n.port
    )
  )

  def channel(contact: Contact): ManagedChannel =
    channels.getOrElseUpdate(
      contact.ip.getHostAddress + ":" + contact.port,
      {
        log.info("Open new channel: {}", contact.ip.getHostAddress + ":" + contact.port)
        ManagedChannelBuilder.forAddress(contact.ip.getHostAddress, contact.port)
          .usePlaintext(true)
          .build
      }
    )

  /**
   * Run some stateful operation, possibly mutating it
   *
   * @param mod Operation
   * @tparam T Return type
   * @return
   */
  override protected def run[T](mod: StateT[Task, RoutingTable[Contact], T], l: String): Task[T] =
    {
      log.info(s"Asking for green light... to perform $l")
      semaphore.greenLight(for {
        rt ← state.take
        _ ← Task.now(log.info("Received RoutingTable, going to update it"))
        rtv ← mod.run(rt)
        _ ← state.put(rtv._1)
      } yield {
        log.info("RoutingTable updated")
        readState.set(rtv._1)
        rtv._2
      })
    }

  override protected def read[T](getter: RoutingTable[Contact] ⇒ T): Task[T] = {
    Task.now(getter(readState.get))
  }

  /**
   * Returns a network wrapper around a contact C, allowing querying it with Kademlia protocol
   *
   * @param contact Description on how to connect to remote node
   * @return
   */
  override def rpc(contact: Contact): KademliaRPC[Task, Contact] = {
    val ch = channel(contact)
    val stub = KademliaGrpc.stub(ch)

    val header = Option(Header(
      from = Some(proto.kademlia.Node(
        id = keyByteString,
        ip = ByteString.copyFrom(ownContact.contact.ip.getAddress),
        port = ownContact.contact.port
      )),
      timestamp = ownContact.lastSeen.toEpochMilli,
      promote = true
    ))

    log.info(s"Going to make rpc call to: ${contact.show}")

    new KademliaRPC[Task, Contact] {
      /**
       * Ping the contact, get its actual Node status, or fail
       *
       * @return
       */
      override def ping(): Task[Node[Contact]] = {
        log.info("Querying ping to: {}", contact.show)
        Task.deferFuture(stub.ping(PingRequest(header)))
          .map(n ⇒ n: Node[Contact])
      }

      /**
       * Perform a local lookup for a key, return K closest known nodes
       *
       * @param key Key to lookup
       * @return
       */
      override def lookup(key: Key, numberOfNodes: Int): Task[Seq[Node[Contact]]] =
        {
          log.info(s"Calling lookup($key, $numberOfNodes) to: {}", contact.show)
          Task.deferFuture(stub.lookup(LookupRequest(header, ByteString.copyFrom(key.id), numberOfNodes)))
            .map(_.nodes.map(n ⇒ n: Node[Contact]))
        }

      /**
       * Perform an iterative lookup for a key, return K closest known nodes
       *
       * @param key Key to lookup
       * @return
       */
      override def lookupIterative(key: Key, numberOfNodes: Int): Task[Seq[Node[Contact]]] =
        {
          if (joined) {
            log.info(s"Calling lookup iterative($key, $numberOfNodes) to: {}", contact.show)
            Task.deferFuture(stub.lookupIterative(LookupRequest(header, ByteString.copyFrom(key.id), numberOfNodes)))
              .map(_.nodes.map(n ⇒ n: Node[Contact]))
          } else {
            lookup(key, numberOfNodes)
          }
        }
    }
  }

  /**
   * How to promote this node to others
   *
   * @return
   */
  override def ownContact: Node[Contact] =
    Node(key, Instant.now(), contact)

}
