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
import fluence.stream.StreamHandler
import monix.execution.Scheduler

/**
 * Contract client for websocket.
 *
 * @param streamHandler Websocket proxy client for grpc.
 */
class ContractsCacheClient[C: ContractValidate](streamHandler: StreamHandler)(
  implicit
  codec: Codec[IO, C, BasicContract],
  checkerFn: CheckerFn,
  ec: Scheduler
) extends ContractsCacheRpc[C] with slogging.LazyLogging {
  import ContractValidate.ContractValidatorOps

  private val keyC = PureCodec.codec[Key, ByteString]

  import fluence.transport.ProtobufCodec._

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
      req ← generatedMessageCodec.runF[IO](FindRequest(idBs))
      responseBytes ← streamHandler.handleUnary(service, "find", req)
      response ← protobufDynamicCodec(BasicContract).runF[IO](responseBytes)
      contract ← codec.decode(response)
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
      req ← generatedMessageCodec.runF[IO](binContract)
      responseBytes ← streamHandler.handleUnary(service, "cache", req)
      resp ← protobufDynamicCodec(CacheResponse).runF[IO](responseBytes)
    } yield resp.cached
}

object ContractsCacheClient {

  def apply[C: ContractValidate](streamHandler: StreamHandler)(
    implicit
    codec: Codec[IO, C, BasicContract],
    checkerFn: CheckerFn,
    ec: Scheduler
  ): ContractsCacheRpc[C] = new ContractsCacheClient(streamHandler)
}
