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

package fluence.contract.grpc.server

import cats.data.Kleisli
import cats.effect.IO
import fluence.codec.Codec
import fluence.contract.protocol.ContractsCacheRpc
import fluence.contract.ops.ContractValidate
import fluence.crypto.SignAlgo.CheckerFn
import fluence.kad.protocol.Key
import fluence.protobuf.contract.{BasicContract, CacheResponse, FindRequest}
import fluence.protobuf.contract.grpc.ContractsCacheGrpc

import scala.concurrent.Future
import scala.language.higherKinds

/**
 * ContractsCache GRPC server implementation.
 *
 * @param cache Delegate implementation
 * @tparam C Domain-level Contract
 */
class ContractsCacheServer[C: ContractValidate](cache: ContractsCacheRpc[C])(
  implicit
  codec: Codec[IO, C, BasicContract],
  checkerFn: CheckerFn,
  keyK: Kleisli[IO, Array[Byte], Key]
) extends ContractsCacheGrpc.ContractsCache {
  import ContractValidate.ContractValidatorOps

  override def find(request: FindRequest): Future[BasicContract] =
    (
      for {
        k ← keyK(request.id.toByteArray)
        resp ← cache.find(k).flatMap[BasicContract] {
          // validating contracts from the local repository is not required
          case Some(c) ⇒ codec.encode(c)
          case None ⇒ IO.raiseError(new NoSuchElementException(""))
        }
      } yield resp
    ).unsafeToFuture()

  override def cache(request: BasicContract): Future[CacheResponse] =
    (
      for {
        contract ← codec.decode(request)
        // we should validate contract before saving in local storage
        _ ← contract.validateME[IO]
        resp ← cache.cache(contract)
      } yield CacheResponse(resp)
    ).unsafeToFuture()
}
