/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.effects.tendermint.rpc.http

import cats.data.EitherT
import cats.effect.{ConcurrentEffect, ContextShift, Timer}
import cats.syntax.apply._
import cats.syntax.either._
import cats.{Functor, Monad}
import com.softwaremill.sttp._
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.effects.tendermint.rpc.response.{Response, TendermintStatus}
import fluence.effects.tendermint.rpc.websocket.{TendermintWebsocketRpc, TendermintWebsocketRpcImpl, WebsocketConfig}
import fluence.log.Log
import io.circe.Json
import io.circe.parser.decode

import scala.language.higherKinds

/**
 * Provides a single concurrent endpoint to run RPC requests on Worker
 *
 * @param host Tendermint hostname (usually docker container name)
 * @param port Tendermint RPC port
 * @tparam F Http requests effect
 */
case class TendermintHttpRpcImpl[F[_]: ConcurrentEffect: Timer: Monad: ContextShift](
  host: String,
  port: Int,
  override val websocketConfig: WebsocketConfig
)(implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing], log: Log[F])
    extends TendermintWebsocketRpcImpl with TendermintHttpRpc[F] with TendermintWebsocketRpc[F] with TendermintRpc[F] {

  val RpcUri = uri"http://$host:$port"
  log.info(s"TendermintRpc created, uri: $RpcUri")

  /** Gets status as a string */
  val status: EitherT[F, RpcError, String] =
    get("status")

  /** Gets status, parse it to [[TendermintStatus]] */
  def statusParsed(implicit F: Functor[F]): EitherT[F, RpcError, TendermintStatus] =
    status
      .map(decode[Response[TendermintStatus]])
      .subflatMap[RpcError, TendermintStatus](
        _.map(_.result).leftMap(RpcBodyMalformed)
      )

  def block(height: Long, id: String = "dontcare"): EitherT[F, RpcError, Block] =
    post(
      RpcRequest(
        method = "block",
        params = Json.fromString(height.toString) :: Nil,
        id = id
      )
    ).subflatMap(str => Block(str).leftMap(RpcBlockParsingFailed(_, str, height)))

  def commit(height: Long, id: String = "dontcare"): EitherT[F, RpcError, String] =
    post(
      RpcRequest(
        method = "commit",
        params = Json.fromString(height.toString) :: Nil,
        id = id
      )
    )

  /**
   * Returns last block height known by this Tendermint node
   */
  def consensusHeight(id: String = "dontcare"): EitherT[F, RpcError, Long] =
    statusParsed.map(_.sync_info.latest_block_height)

  /**
   * Builds a broadcast_tx_commit RPC request
   *
   * NOTE from Tendermint docs: it is not possible to send transactions to Tendermint during `Commit` - if your app tries to send a `/broadcast_tx` to Tendermint during Commit, it will deadlock.
   * TODO: ensure the above deadlock doesn't happen
   *
   * @param tx Transaction body
   * @param id Tracking ID, you may omit it
   */
  def broadcastTxSync(tx: String, id: String): EitherT[F, RpcError, String] =
    post(
      RpcRequest(
        method = "broadcast_tx_sync",
        params = Json.fromString(java.util.Base64.getEncoder.encodeToString(tx.getBytes)) :: Nil,
        id = id
      )
    )

  def unsafeDialPeers(
    peers: Seq[String],
    persistent: Boolean,
    id: String = "dontcare"
  ): EitherT[F, RpcError, String] =
    post(
      RpcRequest(
        method = "dial_peers",
        params =
          Json.arr(peers.map(Json.fromString): _*) :: Json.fromBoolean(persistent) :: Nil,
        id = id
      )
    )

  /** Post a `query` request, wait for response, return it unparsed */
  def query(
    path: String,
    data: String = "",
    height: Long = 0,
    prove: Boolean = false,
    id: String
  ): EitherT[F, RpcError, String] =
    post(
      RpcRequest(
        method = "abci_query",
        params = Json.fromString(path) ::
          Json.fromString(data) ::
          Json.fromString(height.toString) ::
          Json.fromBoolean(prove) :: Nil,
        id = id
      )
    )

  /** Perform the request, and lift the errors to EitherT */
  private def sendHandlingErrors(
    reqT: RequestT[Id, String, Nothing]
  )(implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing]): EitherT[F, RpcError, String] =
    reqT
      .send()
      .leftMap[RpcError](RpcRequestFailed)
      .subflatMap[RpcError, String] { resp ⇒
        val eitherResp = resp.body
          .leftMap[RpcError](RpcRequestErrored(resp.code, _))

        // Print just the first line of response
        log.debug(s"TendermintRpc ${reqT.method.m} response code ${resp.code}")
        log.trace(s"TendermintRpc ${reqT.method.m} full response: $eitherResp")
        eitherResp
      }

  private def logPost(req: RpcRequest): EitherT[F, RpcError, Unit] =
    Log.eitherT[F, RpcError].debug(s"TendermintRpc POST method=${req.method}")

  private def logGet(path: String): EitherT[F, RpcError, Unit] =
    Log.eitherT[F, RpcError].debug(s"TendermintRpc GET path=$path")

  /**
   * Performs a Get request for the given path
   */
  private def get(path: String) = logGet(path) *> sendHandlingErrors(sttp.get(RpcUri.path(path)))

  /**
   * Performs a Post request, sending the given [[RpcRequest]]
   */
  private def post(req: RpcRequest) = logPost(req) *> sendHandlingErrors(sttp.post(RpcUri).body(req.toJsonString))

}
