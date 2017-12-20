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
