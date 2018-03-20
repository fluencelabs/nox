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

import cats.effect.{Async, IO}
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.google.protobuf.ByteString
import fluence.codec.Codec
import fluence.contract.protocol.ContractsCacheRpc
import fluence.contract.grpc.ContractsCacheGrpc.ContractsCacheStub
import fluence.contract.grpc.{BasicContract, ContractsCacheGrpc, FindRequest}
import fluence.kad.protocol.Key
import fluence.codec.pb.ProtobufCodecs._
import io.grpc.{CallOptions, ManagedChannel}

import scala.concurrent.{ExecutionContext, Future}
import scala.language.higherKinds

class ContractsCacheClient[F[_]: Async, C](stub: ContractsCacheStub)(
  implicit
  codec: Codec[F, C, BasicContract],
  ec: ExecutionContext
) extends ContractsCacheRpc[F, C] {

  private def run[A](fa: Future[A]): F[A] = IO.fromFuture(IO(fa)).to[F]

  /**
   * Tries to find a contract in local cache.
   *
   * @param id Dataset ID
   * @return Optional locally found contract
   */
  override def find(id: Key): F[Option[C]] =
    (for {
      idBs ← Codec.codec[F, ByteString, Key].decode(id)
      req = FindRequest(idBs)
      resRaw ← run(stub.find(req))
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
  override def cache(contract: C): F[Boolean] =
    for {
      c ← codec.encode(contract)
      resp ← run(stub.cache(c))
    } yield resp.cached
}

object ContractsCacheClient {

  /**
   * Shorthand to register inside NetworkClient.
   *
   * @param channel     Channel to remote node
   * @param callOptions Call options
   */
  def register[F[_]: Async, C]()(
    channel: ManagedChannel,
    callOptions: CallOptions
  )(
    implicit
    codec: Codec[F, C, BasicContract],
    ec: ExecutionContext
  ): ContractsCacheRpc[F, C] =
    new ContractsCacheClient[F, C](new ContractsCacheGrpc.ContractsCacheStub(channel, callOptions))
}
