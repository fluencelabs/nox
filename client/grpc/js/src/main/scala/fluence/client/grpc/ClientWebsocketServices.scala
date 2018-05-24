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

object ClientWebsocketServices {

  val builder: String ⇒ WebsocketT = str ⇒ Websocket(str)

  def build[F[_]: Effect](
    implicit
    checkerFn: CheckerFn,
    scheduler: Scheduler = Scheduler.global
  ): Contact ⇒ ClientServices[F, BasicContract, Contact] = {
    import fluence.contract.grpc.BasicContractCodec.{codec ⇒ contractCodec}
    import fluence.kad.KademliaNodeCodec.{pureCodec ⇒ nodeCodec}

    contact ⇒
      {

        val url = "ws://" + "127.0.0.1" + ":" + contact.websocketPort.get

        new ClientServices[F, BasicContract, Contact] {
          override def kademlia: KademliaRpc[Contact] =
            new KademliaWebsocketClient(ConnectionPool.getOrCreateConnection(url, builder))

          override def contractsCache: ContractsCacheRpc[BasicContract] =
            new ContractsCacheClient[BasicContract](ConnectionPool.getOrCreateConnection(url, builder))

          override def contractAllocator: ContractAllocatorRpc[BasicContract] =
            new ContractAllocatorClient[BasicContract](ConnectionPool.getOrCreateConnection(url, builder))
          // todo generalize Observable
          override def datasetStorage: DatasetStorageRpc[F, Observable] =
            new DatasetStorageClient(ConnectionPool.getOrCreateConnection(url, builder))
        }
      }
  }
}
