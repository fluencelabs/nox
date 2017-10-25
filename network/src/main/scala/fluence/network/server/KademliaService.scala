package fluence.network.server

import java.time.Instant

import cats.data.StateT
import fluence.kad._
import fluence.network.Contact
import fluence.network.client.KademliaClient
import monix.eval.{ MVar, Task, TaskSemaphore }
import monix.execution.atomic.AtomicAny
import org.slf4j.LoggerFactory

import scala.concurrent.duration._
import scala.language.implicitConversions

class KademliaService(
    override val key: Key,
    contact:          Task[Contact],
    client:           Contact ⇒ KademliaClient,
    k:                Int,
    alpha:            Int                      = 3,
    pingTimeout:      Duration                 = 1.second
) extends Kademlia[Task, Contact](Alpha = alpha, K = k, pingTimeout = pingTimeout) {

  private val readState = AtomicAny(RoutingTable[Contact](key, K, K))

  private val state = MVar(readState.get)
  private val semaphore = TaskSemaphore(1)

  private val log = LoggerFactory.getLogger(getClass)

  /**
   * Run some stateful operation, possibly mutating it
   *
   * @param mod Operation
   * @tparam T Return type
   * @return
   */
  override protected def run[T](mod: StateT[Task, RoutingTable[Contact], T], l: String): Task[T] = {
    log.info(s"$key / Asking for green light... to perform $l")
    semaphore.greenLight(for {
      rt ← state.take
      _ ← Task.now(log.info("Received RoutingTable, going to update it"))
      rtv ← mod.run(rt)
      _ ← state.put(rtv._1)
    } yield {
      log.info(s"$key / RoutingTable updated")
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
  override def rpc(contact: Contact): KademliaRPC[Task, Contact] = client(contact)

  /**
   * How to promote this node to others
   *
   * @return
   */
  override def ownContact: Task[Node[Contact]] =
    contact.map(c ⇒ Node(key, Instant.now(), c))

}
