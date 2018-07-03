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
import fluence.contract.grpc.client.{ContractAllocatorClientGrpc, ContractsCacheClientGrpc}
import fluence.contract.protobuf.grpc.{ContractAllocatorGrpc, ContractsCacheGrpc}
import fluence.contract.protocol.{ContractAllocatorRpc, ContractsCacheRpc}
import fluence.crypto.signature.SignAlgo.CheckerFn
import fluence.dataset.grpc.client.DatasetStorageClientGrpc
import fluence.dataset.protobuf.grpc.DatasetStorageRpcGrpc
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.grpc.ServiceManager
import fluence.kad.grpc.client.KademliaClientGrpc
import fluence.kad.protobuf.grpc.KademliaGrpc
import fluence.kad.protocol.{Contact, KademliaRpc}
import fluence.transport.grpc.client.GrpcClient
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

    val client = builder
      .add(KademliaClientGrpc.register(serviceManager))
      .add(ContractsCacheClientGrpc.register[BasicContract](serviceManager))
      .add(ContractAllocatorClientGrpc.register[BasicContract](serviceManager))
      .add(DatasetStorageClientGrpc.register[F](serviceManager))
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
