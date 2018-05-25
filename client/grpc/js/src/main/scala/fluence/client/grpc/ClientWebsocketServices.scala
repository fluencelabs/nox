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
import fluence.transport.websocket.{ConnectionPool, Websocket, WebsocketT}
import monix.execution.Scheduler
import monix.reactive.Observable

import scala.language.higherKinds

class ClientWebsocketServices(connectionPool: ConnectionPool) {

  val builder: String ⇒ WebsocketT = Websocket.builder

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
              new KademliaWebsocketClient(connectionPool.getOrCreateConnection(url, builder))

            override def contractsCache: ContractsCacheRpc[BasicContract] =
              new ContractsCacheClient[BasicContract](connectionPool.getOrCreateConnection(url, builder))

            override def contractAllocator: ContractAllocatorRpc[BasicContract] =
              new ContractAllocatorClient[BasicContract](connectionPool.getOrCreateConnection(url, builder))
            // todo generalize Observable
            override def datasetStorage: DatasetStorageRpc[F, Observable] =
              new DatasetStorageClient(connectionPool.getOrCreateConnection(url, builder))
          }
        }
      }
  }
}
