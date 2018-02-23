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

package fluence.client

import cats.effect.Effect
import fluence.crypto.signature.SignatureChecker
import fluence.dataset.BasicContract
import fluence.dataset.grpc.DatasetStorageClient
import fluence.dataset.grpc.client.{ ContractAllocatorClient, ContractsCacheClient }
import fluence.kad.grpc.client.KademliaClient
import fluence.transport.grpc.client.GrpcClient
import monix.execution.Scheduler
import shapeless.HNil

import scala.language.higherKinds

object ClientComposer {

  /**
   * Register all Rpc's into [[fluence.transport.TransportClient]] and returns it.
   */
  def grpc[F[_] : Effect](
    builder: GrpcClient.Builder[HNil]
  )(implicit checker: SignatureChecker, scheduler: Scheduler = Scheduler.global) = {

    import fluence.dataset.grpc.BasicContractCodec.{ codec ⇒ contractCodec }
    import fluence.kad.grpc.KademliaNodeCodec.{ codec ⇒ nodeCodec }

    builder
      .add(KademliaClient.register[F]())
      .add(ContractsCacheClient.register[F, BasicContract]())
      .add(ContractAllocatorClient.register[F, BasicContract]())
      .add(DatasetStorageClient.register[F]())
      .build
  }

} //0x037833305d7a40fa389503d0783d4d2b0d0470d875c10ffbcaa306b59a12f82c0c
