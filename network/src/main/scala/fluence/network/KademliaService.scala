package fluence.network

import java.net.InetAddress
import java.time.Instant

import com.google.protobuf.ByteString
import fluence.kad.{ Kademlia, Key }
import fluence.network.proto.kademlia._
import monix.eval.Task
import monix.execution.Scheduler
import org.slf4j.LoggerFactory

import scala.concurrent.Future
import scala.language.implicitConversions

class KademliaService(kad: Kademlia[Task, Contact])(implicit sc: Scheduler) extends KademliaGrpc.Kademlia {
  private val log = LoggerFactory.getLogger(getClass)

  private implicit def ncToNode(nc: fluence.kad.Node[Contact]): Node =
    Node(id = ByteString.copyFrom(nc.key.id), ByteString.copyFrom(nc.contact.ip.getAddress), nc.contact.port)

  def update[T](header: Option[Header], r: T): Task[T] = {
    (for {
      h ← header if h.promote
      n ← h.from
    } yield fluence.kad.Node[Contact](
      Key(n.id.toByteArray),
      Instant.ofEpochMilli(h.timestamp),
      Contact(
        InetAddress.getByAddress(n.ip.toByteArray),
        n.port
      )
    )).fold(Task.unit)(kad.update).map(_ ⇒ r)
  }

  override def ping(request: PingRequest): Future[Node] =
    {
      log.info("Incoming ping")
      kad.handleRPC
        .ping()
        .map(nc ⇒ nc: Node)
        .map{ n ⇒ log.info(s"Ping reply: {}", n); n }
        .flatMap(update(request.header, _))

        .onErrorRecoverWith{
          case e ⇒
            log.error("Can't reply on ping!", e)
            Task.raiseError(e)
        }.runAsync
    }

  override def lookup(request: LookupRequest): Future[NodesResponse] =
    {
      log.info(s"Incoming lookup: $request")

      kad.handleRPC
        .lookup(Key(request.key.toByteArray), request.numberOfNodes)
        .map(_.map(nc ⇒ nc: Node))
        .map(NodesResponse(_))
        .map{ n ⇒ log.info(s"Lookup reply: {}", n); n }
        .flatMap(update(request.header, _))

        .onErrorRecoverWith{
          case e ⇒
            log.error("Can't reply on lookup!", e)
            Task.raiseError(e)
        }
        .runAsync
    }

  override def lookupIterative(request: LookupRequest): Future[NodesResponse] =
    {
      log.info(s"Incoming lookup iterative: $request")

      kad.handleRPC
        .lookupIterative(Key(request.key.toByteArray), request.numberOfNodes)
        .map(_.map(nc ⇒ nc: Node))
        .map(NodesResponse(_))
        .map{ n ⇒ log.info(s"reply li: $n"); n }
        .flatMap(update(request.header, _))

        .onErrorRecoverWith{
          case e ⇒
            log.error("can't reply on lookup iterative!", e)
            Task.raiseError(e)
        }
        .runAsync
    }
}
