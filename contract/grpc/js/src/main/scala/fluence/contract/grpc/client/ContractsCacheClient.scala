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
import fluence.codec.{Codec, PureCodec}
import fluence.contract.ops.ContractValidate
import fluence.contract.protobuf.{BasicContract, CacheResponse, FindRequest}
import fluence.contract.protocol.ContractsCacheRpc
import fluence.crypto.signature.SignAlgo.CheckerFn
import fluence.kad.KeyProtobufCodecs._
import fluence.kad.protocol.Key
import fluence.proxy.grpc.WebsocketMessage
import fluence.transport.websocket.{ConnectionPool, GrpcProxyClient, WebsocketPipe}
import monix.execution.Scheduler

/**
 * Contract client for websocket.
 *
 * @param connection Websocket pipe.
 */
class ContractsCacheClient[C: ContractValidate](connection: IO[WebsocketPipe[WebsocketMessage, WebsocketMessage]])(
  implicit
  codec: Codec[IO, C, BasicContract],
  checkerFn: CheckerFn,
  ec: Scheduler
) extends ContractsCacheRpc[C] with slogging.LazyLogging {
  import ContractValidate.ContractValidatorOps

  private val keyC = PureCodec.codec[Key, ByteString]

  import fluence.transport.websocket.ProtobufCodec._

  private val service = "fluence.contract.protobuf.grpc.ContractsCache"

  /**
   * Tries to find a contract in local cache.
   *
   * @param id Dataset ID
   * @return Optional locally found contract
   */
  override def find(id: Key): IO[Option[C]] =
    (for {
      idBs ← keyC.direct.runF[IO](id)
      websocket ← connection
      req = FindRequest(idBs)
      proxy = GrpcProxyClient
        .proxy(service, "find", websocket, generatedMessageCodec, protobufDynamicCodec(BasicContract))
      binContract ← IO.fromFuture(IO(proxy.requestAndWaitOneResult(req)))
      contract ← codec.decode(binContract)
      // contract from the outside required validation
      _ ← contract.validateME[IO]
    } yield Option(contract)).recover {
      case err ⇒
        logger.warn(s"Finding contract failed, cause=$err", err)
        None
    }

  /**
   * Ask node to cache the contract.
   *
   * @param contract Contract to cache
   * @return If the contract is cached or not
   */
  override def cache(contract: C): IO[Boolean] =
    for {
      // we should validate contract before send outside to caching
      _ ← contract.validateME[IO]
      binContract ← codec.encode(contract)
      websocket ← connection
      proxy = GrpcProxyClient
        .proxy(service, "cache", websocket, generatedMessageCodec, protobufDynamicCodec(CacheResponse))
      resp ← IO.fromFuture(IO(proxy.requestAndWaitOneResult(binContract)))
    } yield resp.cached
}
