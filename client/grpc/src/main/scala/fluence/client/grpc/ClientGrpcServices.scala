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

import cats.Applicative
import cats.effect.Effect
import fluence.client.core.ClientServices
import fluence.codec.Codec
import fluence.contract.BasicContract
import fluence.contract.grpc.client.{ ContractAllocatorClient, ContractsCacheClient }
import fluence.contract.protocol.{ ContractAllocatorRpc, ContractsCacheRpc }
import fluence.crypto.signature.SignatureChecker
import fluence.dataset.grpc.DatasetStorageClient
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.protocol.{ Contact, KademliaRpc }
import fluence.transport.grpc.client.{ GrpcClient, GrpcError }
import monix.execution.Scheduler
import shapeless.HNil

import scala.language.higherKinds

object ClientGrpcServices {
  type Error = Throwable

  implicit def errorKadCodec[F[_] : Applicative]: Codec[F, Error, fluence.kad.grpc.Error] =
    Codec.pure(
      throwable ⇒ fluence.kad.grpc.Error(throwable.getMessage),
      remote ⇒ new RuntimeException(remote.message): Throwable
    )

  def build[F[_] : Effect](
    builder: GrpcClient.Builder[HNil]
  )(
    implicit
    checker: SignatureChecker,
    scheduler: Scheduler = Scheduler.global
  ): Contact ⇒ ClientServices[F, BasicContract, Contact, Error] = {
    import fluence.contract.grpc.BasicContractCodec.{ codec ⇒ contractCodec }
    import fluence.kad.grpc.KademliaNodeCodec.{ codec ⇒ nodeCodec }
    implicit val convert: GrpcError ⇒ Error = _.cause
    import fluence.transport.grpc.client.GrpcRunner.runner

    val client = builder
      .add(KademliaClient.register[F, Error]())
      .add(ContractsCacheClient.register[F, BasicContract]())
      .add(ContractAllocatorClient.register[F, BasicContract]())
      .add(DatasetStorageClient.register[F]())
      .build

    contact ⇒ new ClientServices[F, BasicContract, Contact, Error] {
      override def kademlia: KademliaRpc.Aux[F, Contact, Error] =
        client.service[KademliaRpc.Aux[F, Contact, Error]](contact)

      override def contractsCache: ContractsCacheRpc[F, BasicContract] =
        client.service[ContractsCacheRpc[F, BasicContract]](contact)

      override def contractAllocator: ContractAllocatorRpc[F, BasicContract] =
        client.service[ContractAllocatorRpc[F, BasicContract]](contact)

      override def datasetStorage: DatasetStorageRpc[F] =
        client.service[DatasetStorageRpc[F]](contact)
    }
  }
}
