package fluence.btree.server

import fluence.btree.client.merkle.MerkleRootCalculator
import fluence.btree.client.network._
import fluence.btree.server.MerkleBTreeServer._
import fluence.btree.server.network.BTreeServerNetwork
import fluence.btree.server.network.BTreeServerNetwork.RequestHandler
import fluence.btree.server.network.commands.{ GetCommandImpl, PutCommandImpl }
import monix.eval.Task
import org.slf4j.LoggerFactory

/**
 * Server for [[MerkleBTree]] for to interaction with [[fluence.btree.client.MerkleBTreeClient]].
 *
 * @param btree             MerkleBTree engine.
 * @param networkProvider  Factory for immutable network initialization for any request handler
 */
class MerkleBTreeServer(
    btree: MerkleBTree,
    merkleRootCalculator: MerkleRootCalculator,
    networkProvider: RequestHandler[Task] ⇒ BTreeServerNetwork[Task]
) {

  /**
   * Handler for every initialization request from client to server.
   * Will be invoked by [[BTreeServerNetwork]] for each initialization request received from BTree client.
   */
  private val InitRequestHandler: RequestHandler[Task] = {
    case (InitGetRequest, askRequiredDetails: (BTreeServerResponse ⇒ Task[Option[BTreeClientRequest]])) ⇒
      val command = new GetCommandImpl[Task](askRequiredDetails)
      btree.get(command)

    case (InitPutRequest, askRequiredDetails: (BTreeServerResponse ⇒ Task[Option[BTreeClientRequest]])) ⇒
      val command = new PutCommandImpl[Task](merkleRootCalculator, askRequiredDetails)
      btree.put(command)
  }

  private lazy val network = networkProvider(InitRequestHandler)

  /** Start the MerkleBTreeServer. */
  def start(): Task[Boolean] = {
    log.debug(s"${getClass.getSimpleName} starts")
    network.start()
  }

  /** Shut the MerkleBTreeServer down. */
  def shutdown(): Task[Boolean] = {
    log.debug(s"${getClass.getSimpleName} was shut down")
    network.shutdown()
  }

}

object MerkleBTreeServer {

  private val log = LoggerFactory.getLogger(getClass)

  def apply(
    btree: MerkleBTree,
    merkleRootCalculator: MerkleRootCalculator,
    networkProvider: RequestHandler[Task] ⇒ BTreeServerNetwork[Task]
  ): MerkleBTreeServer = new MerkleBTreeServer(btree, merkleRootCalculator, networkProvider)

}
