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

package fluence.client.grpc

import cats.effect.Effect
import fluence.client.core.ClientServices
import fluence.contract.BasicContract
import fluence.contract.grpc.client.{ContractAllocatorClient, ContractsCacheClient}
import fluence.contract.protocol.{ContractAllocatorRpc, ContractsCacheRpc}
import fluence.crypto.signature.SignAlgo.CheckerFn
import fluence.dataset.grpc.client.DatasetStorageClient
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.kad.grpc.client.KademliaWebsocketClient
import fluence.kad.protocol.{Contact, KademliaRpc}
import fluence.proxy.grpc.WebsocketMessage
import fluence.transport.websocket.{ConnectionPool, Websocket, WebsocketT}
import monix.execution.Scheduler
import monix.reactive.Observable

import scala.language.higherKinds

/**
 * Client services that can interact with nodes by websocket connections.
 *
 * @param connectionPool Pool of websocket connections.
 */
class ClientWebsocketServices(connectionPool: ConnectionPool[WebsocketMessage, WebsocketMessage]) {

  def build[F[_]: Effect](
    implicit
    checkerFn: CheckerFn,
    scheduler: Scheduler = Scheduler.global
  ): Contact ⇒ Option[ClientServices[F, BasicContract, Contact]] = {
    import fluence.contract.grpc.BasicContractCodec.{codec ⇒ contractCodec}
    import fluence.kad.KademliaNodeCodec.{pureCodec ⇒ nodeCodec}

    contact ⇒
      {
        contact.websocketPort.map { wsPort ⇒
          val url = "ws://" + contact.addr + ":" + wsPort

          new ClientServices[F, BasicContract, Contact] {
            override def kademlia: KademliaRpc[Contact] =
              new KademliaWebsocketClient(connectionPool.getOrCreateConnection(url))

            override def contractsCache: ContractsCacheRpc[BasicContract] =
              new ContractsCacheClient[BasicContract](url, connectionPool)

            override def contractAllocator: ContractAllocatorRpc[BasicContract] =
              new ContractAllocatorClient[BasicContract](url, connectionPool)
            // todo generalize Observable
            override def datasetStorage: DatasetStorageRpc[F, Observable] =
              new DatasetStorageClient(url, connectionPool)
          }
        }
      }
  }
}
