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

import cats.effect.{Effect, IO}
import fluence.client.core.ClientServices
import fluence.contract.BasicContract
import fluence.contract.grpc.client.{ContractAllocatorClient, ContractsCacheClient}
import fluence.contract.protobuf.grpc.{ContractAllocatorGrpc, ContractsCacheGrpc}
import fluence.contract.protocol.{ContractAllocatorRpc, ContractsCacheRpc}
import fluence.crypto.signature.SignAlgo.CheckerFn
import fluence.dataset.grpc.client.DatasetStorageClient
import fluence.dataset.protobuf.grpc.DatasetStorageRpcGrpc
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.grpc.{GrpcConnection, ServiceManager}
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.protobuf.grpc.KademliaGrpc
import fluence.kad.protocol.{Contact, KademliaRpc}
import fluence.transport.grpc.client.GrpcClient
import io.grpc.{CallOptions, ManagedChannel}
import monix.execution.Scheduler
import monix.reactive.Observable
import shapeless.HNil

import scala.language.higherKinds

object ClientGrpcServices {

  def build[F[_]: Effect](
    builder: GrpcClient.Builder[HNil]
  )(
    implicit
    checkerFn: CheckerFn,
    scheduler: Scheduler = Scheduler.global
  ): Contact ⇒ ClientServices[F, BasicContract, Contact] = {
    import fluence.contract.grpc.BasicContractCodec.{codec ⇒ contractCodec}
    import fluence.kad.KademliaNodeCodec.{pureCodec ⇒ nodeCodec}

    val services = List(
      KademliaGrpc.SERVICE,
      ContractsCacheGrpc.SERVICE,
      ContractAllocatorGrpc.SERVICE,
      DatasetStorageRpcGrpc.SERVICE
    )

    val serviceManager = ServiceManager(services)
    val handlerBuilder: IO[(ManagedChannel, CallOptions)] => GrpcConnection = GrpcConnection.builder(serviceManager)

    val client = builder
      .add(handlerBuilder andThen KademliaClient.apply)
      .add(handlerBuilder andThen ContractsCacheClient.apply[BasicContract])
      .add(handlerBuilder andThen ContractAllocatorClient.apply[BasicContract])
      .add(handlerBuilder andThen DatasetStorageClient.apply[F])
      .build

    contact ⇒
      new ClientServices[F, BasicContract, Contact] {
        override def kademlia: KademliaRpc[Contact] =
          client.service[KademliaRpc[Contact]](contact)

        override def contractsCache: ContractsCacheRpc[BasicContract] =
          client.service[ContractsCacheRpc[BasicContract]](contact)

        override def contractAllocator: ContractAllocatorRpc[BasicContract] =
          client.service[ContractAllocatorRpc[BasicContract]](contact)
        // todo generalize Observable
        override def datasetStorage: DatasetStorageRpc[F, Observable] =
          client.service[DatasetStorageRpc[F, Observable]](contact)
      }
  }
}
