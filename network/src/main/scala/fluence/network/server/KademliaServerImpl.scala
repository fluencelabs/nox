package fluence.network.server

import java.net.InetAddress
import java.time.Instant

import com.google.protobuf.ByteString
import fluence.kad.{ Kademlia, Key }
import fluence.network.Contact
import fluence.network.proto.kademlia._
import monix.eval.Task
import monix.execution.Scheduler
import org.slf4j.LoggerFactory
import cats.syntax.eq._

import scala.concurrent.Future
import scala.language.implicitConversions

class KademliaServerImpl(kad: Kademlia[Task, Contact])(implicit sc: Scheduler) extends KademliaGrpc.Kademlia {
  private val log = LoggerFactory.getLogger(getClass)

  private implicit def ncToNode(nc: fluence.kad.Node[Contact]): Node =
    Node(id = ByteString.copyFrom(nc.key.id), ByteString.copyFrom(nc.contact.ip.getAddress), nc.contact.port)

  private def isFromSelf(optHeader: Option[Header]): Boolean =
    optHeader
      .flatMap(_.from)
      .map(_.id)
      .filter(_.size() == Key.Length)
      .map(bs ⇒ Key(bs.toByteArray))
      .exists(_ === kad.key)

  private def runIfNotSelf[T](optHeader: Option[Header], task: Task[T]): Future[T] =
    if (isFromSelf(optHeader)) {
      log.warn("Dropping request as it's from self node")
      Future.failed[T](new IllegalArgumentException("Can't handle requests from self"))
    } else {
      update(optHeader) // no need to track results
      task.runAsync
    }

  def update(header: Option[Header]): Unit = {
    (for {
      h ← header if h.advertize
      n ← h.from
    } yield fluence.kad.Node[Contact](
      Key(n.id.toByteArray),
      Instant.now(),
      Contact(
        InetAddress.getByAddress(n.ip.toByteArray),
        n.port
      )
    )).fold(Task.unit)(kad.update).runAsync
  }

  override def ping(request: PingRequest): Future[Node] =
    {
      log.info(s"${kad.key} / Incoming ping: from {}", request.header.flatMap(_.from).map(_.id.toByteArray).map(Key(_)))

      runIfNotSelf(
        request.header,
        kad.handleRPC
          .ping()
          .map(nc ⇒ nc: Node)
          .map{ n ⇒ log.info(s"Ping reply: {}", n); n }

          .onErrorRecoverWith{
            case e ⇒
              log.error("Can't reply on ping!", e)
              Task.raiseError(e)
          }
      )
    }

  override def lookup(request: LookupRequest): Future[NodesResponse] =
    {
      log.info(s"${kad.key} / Incoming lookup: for {}, from {}", Key(request.key.toByteArray), request.header.flatMap(_.from).map(_.id.toByteArray).map(Key(_)))

      runIfNotSelf(
        request.header,
        kad.handleRPC
          .lookup(Key(request.key.toByteArray), request.numberOfNodes)
          .map{ n ⇒ log.info(s"Lookup reply: {}", n); n }
          .map(_.map(nc ⇒ nc: Node))
          .map(NodesResponse(_))

          .onErrorRecoverWith{
            case e ⇒
              log.error("Can't reply on lookup!", e)
              Task.raiseError(e)
          }
      )
    }

  override def lookupIterative(request: LookupRequest): Future[NodesResponse] =
    {
      log.info(s"${kad.key} / Incoming lookup iterative: for {}, from {}", Key(request.key.toByteArray), request.header.flatMap(_.from).map(_.id.toByteArray).map(Key(_)))

      runIfNotSelf(
        request.header,
        kad.handleRPC
          .lookupIterative(Key(request.key.toByteArray), request.numberOfNodes)
          .map{ n ⇒ log.info(s"Reply to lookupIterative: $n"); n }
          .map(_.map(nc ⇒ nc: Node))
          .map(NodesResponse(_))

          .onErrorRecoverWith{
            case e ⇒
              log.error("can't reply on lookup iterative!", e)
              Task.raiseError(e)
          }
      )
    }
}
