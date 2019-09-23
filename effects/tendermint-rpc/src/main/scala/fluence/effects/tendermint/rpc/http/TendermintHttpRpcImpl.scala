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
import cats.syntax.apply._
import cats.syntax.either._
import cats.{Functor, Monad}
import com.softwaremill.sttp._
import com.softwaremill.sttp.circe._
import fluence.bp.tx.TxResponse
import fluence.effects.sttp.SttpEffect
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.response.{Response, TendermintStatus}
import fluence.log.Log
import io.circe
import io.circe.{Decoder, Json}
import io.circe.parser.decode

import scala.language.higherKinds

/**
 * Provides a single concurrent endpoint to run RPC requests on Worker
 *
 * @param host Tendermint hostname (usually docker container name)
 * @param port Tendermint RPC port
 * @tparam F Http requests effect
 */
case class TendermintHttpRpcImpl[F[_]: Monad: SttpEffect](
  host: String,
  port: Int
) extends TendermintHttpRpc[F] {

  val RpcUri = uri"http://$host:$port"

  /** Gets status as a string */
  def status(implicit log: Log[F]): EitherT[F, RpcError, String] =
    get("status")

  /** Gets status, parse it to [[TendermintStatus]] */
  def statusParsed(implicit F: Functor[F], log: Log[F]): EitherT[F, RpcError, TendermintStatus] =
    status
      .map(decode[Response[TendermintStatus]])
      .subflatMap[RpcError, TendermintStatus](
        _.map(_.result).leftMap(RpcBodyMalformed)
      )

  def block(height: Long, id: String = "dontcare")(implicit log: Log[F]): EitherT[F, RpcError, Block] =
    post(
      RpcRequest(
        method = "block",
        params = Json.fromString(height.toString) :: Nil,
        id = id
      )
    ).subflatMap(str => Block(str).leftMap(RpcBlockParsingFailed(_, str, height)))

  def commit(height: Long, id: String = "dontcare")(implicit log: Log[F]): EitherT[F, RpcError, String] =
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
  def consensusHeight(id: String = "dontcare")(implicit log: Log[F]): EitherT[F, RpcError, Long] =
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
  def broadcastTxSync(tx: Array[Byte], id: String = "dontcare")(
    implicit log: Log[F]
  ): EitherT[F, RpcError, TxResponse] =
    postT[TxResponse](
      RpcRequest(
        method = "broadcast_tx_sync",
        params = Json.fromString(java.util.Base64.getEncoder.encodeToString(tx)) :: Nil,
        id = id
      )
    )

  def unsafeDialPeers(
    peers: Seq[String],
    persistent: Boolean,
    id: String = "dontcare"
  )(implicit log: Log[F]): EitherT[F, RpcError, String] =
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
  )(implicit log: Log[F]): EitherT[F, RpcError, String] =
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
  private def sendHandlingErrors[T](
    reqT: RequestT[Id, T, Nothing]
  )(implicit log: Log[F]): EitherT[F, RpcError, T] =
    reqT
      .send()
      .leftMap[RpcError](RpcRequestFailed)
      .subflatMap[RpcError, T] { resp ⇒
        val eitherResp = resp.body.leftMap[RpcError](RpcHttpError(resp.code, _))

        // Print just the first line of response
        // TODO really? it does nothing
        log.debug(s"TendermintRpc ${reqT.method.m} response code ${resp.code}")
        log.trace(s"TendermintRpc ${reqT.method.m} full response: $eitherResp")
        eitherResp
      }

  private def logPost(req: RpcRequest)(implicit log: Log[F]): EitherT[F, RpcError, Unit] =
    Log.eitherT[F, RpcError].debug(s"TendermintRpc POST method=${req.method}")

  private def logGet(path: String)(implicit log: Log[F]): EitherT[F, RpcError, Unit] =
    Log.eitherT[F, RpcError].debug(s"TendermintRpc GET path=$path")

  /**
   * Performs a Get request for the given path
   */
  private def get(path: String)(implicit log: Log[F]) = logGet(path) *> sendHandlingErrors(sttp.get(RpcUri.path(path)))

  /**
   * Performs a Post request, sending the given [[RpcRequest]]
   */
  private def postT[T: Decoder](req: RpcRequest)(implicit log: Log[F]): EitherT[F, RpcError, T] = {
    import RpcCallError._
    val request = sttp.post(RpcUri).body(req.toJsonString).response(asJson[Either[RpcCallError, T]])
    logPost(req) *> sendHandlingErrors(request).subflatMap(_.leftMap(e => RpcBodyMalformed(e.error)).flatMap(identity))
  }

  private def post(req: RpcRequest)(implicit log: Log[F]): EitherT[F, RpcError, String] = postT[String](req)
}
