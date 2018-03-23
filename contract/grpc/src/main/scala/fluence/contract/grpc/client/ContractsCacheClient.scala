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

package fluence.contract.grpc.client

import cats.effect.IO
import cats.syntax.applicativeError._
import com.google.protobuf.ByteString
import fluence.codec.Codec
import fluence.contract.protocol.ContractsCacheRpc
import fluence.contract.grpc.ContractsCacheGrpc.ContractsCacheStub
import fluence.contract.grpc.{BasicContract, ContractsCacheGrpc, FindRequest}
import fluence.kad.protocol.Key
import fluence.codec.pb.ProtobufCodecs._
import io.grpc.{CallOptions, ManagedChannel}

import scala.concurrent.ExecutionContext

class ContractsCacheClient[C](stub: ContractsCacheStub)(
  implicit
  codec: Codec[IO, C, BasicContract],
  ec: ExecutionContext
) extends ContractsCacheRpc[C] {

  /**
   * Tries to find a contract in local cache.
   *
   * @param id Dataset ID
   * @return Optional locally found contract
   */
  override def find(id: Key): IO[Option[C]] =
    (for {
      idBs ← Codec.codec[IO, ByteString, Key].decode(id)
      req = FindRequest(idBs)
      resRaw ← IO.fromFuture(IO(stub.find(req)))
      res ← codec.decode(resRaw)
    } yield Option(res)).recover {
      case _ ⇒ None
    }

  /**
   * Ask node to cache the contract.
   *
   * @param contract Contract to cache
   * @return If the contract is cached or not
   */
  override def cache(contract: C): IO[Boolean] =
    for {
      c ← codec.encode(contract)
      resp ← IO.fromFuture(IO(stub.cache(c)))
    } yield resp.cached
}

object ContractsCacheClient {

  /**
   * Shorthand to register inside NetworkClient.
   *
   * @param channel     Channel to remote node
   * @param callOptions Call options
   */
  def register[C]()(
    channel: ManagedChannel,
    callOptions: CallOptions
  )(
    implicit
    codec: Codec[IO, C, BasicContract],
    ec: ExecutionContext
  ): ContractsCacheRpc[C] =
    new ContractsCacheClient[C](new ContractsCacheGrpc.ContractsCacheStub(channel, callOptions))
}
