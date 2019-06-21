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

package fluence.effects.tendermint.rpc

import cats.Functor
import cats.data.EitherT
import cats.effect.{Resource, Sync}
import cats.syntax.apply._
import cats.syntax.functor._
import cats.syntax.either._
import com.softwaremill.sttp._
import fluence.effects.tendermint.rpc.response.{Response, TendermintStatus}
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
case class TendermintRpc[F[_]: Sync](
  host: String,
  port: Int
)(implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing])
    extends WebsocketTendermintRpc {

  val rpcUri = uri"http://$host:$port"

  /** Get status as string */
  def status(implicit log: Log[F]): EitherT[F, RpcError, String] =
    get("status")

  /** Get status, parse it to [[TendermintStatus]] */
  def statusParsed(implicit F: Functor[F], log: Log[F]): EitherT[F, RpcError, TendermintStatus] =
    status
      .map(decode[Response[TendermintStatus]])
      .subflatMap[RpcError, TendermintStatus](
        _.map(_.result).leftMap(RpcBodyMalformed)
      )

  def block(height: Long, id: String = "dontcare")(implicit log: Log[F]): EitherT[F, RpcError, String] =
    post(
      RpcRequest(
        method = "block",
        params = Json.fromString(height.toString) :: Nil,
        id = id
      )
    )

  def commit(height: Long, id: String = "dontcare")(implicit log: Log[F]): EitherT[F, RpcError, String] =
    post(
      RpcRequest(
        method = "commit",
        params = Json.fromString(height.toString) :: Nil,
        id = id
      )
    )

  /**
   * Builds a broadcast_tx_commit RPC request
   *
   * NOTE from Tendermint docs: it is not possible to send transactions to Tendermint during `Commit` - if your app tries to send a `/broadcast_tx` to Tendermint during Commit, it will deadlock.
   * TODO: ensure the above deadlock doesn't happen
   *
   * @param tx Transaction body
   * @param id Tracking ID, you may omit it
   */
  def broadcastTxSync(tx: String, id: String)(implicit log: Log[F]): EitherT[F, RpcError, String] =
    post(
      RpcRequest(
        method = "broadcast_tx_sync",
        params = Json.fromString(java.util.Base64.getEncoder.encodeToString(tx.getBytes)) :: Nil,
        id = id
      )
    )

  def unsafeDialPeers(peers: Seq[String], persistent: Boolean, id: String = "dontcare")(
    implicit log: Log[F]
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
  private def sendHandlingErrors(
    reqT: RequestT[Id, String, Nothing]
  )(implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing], log: Log[F]): EitherT[F, RpcError, String] =
    reqT
      .send()
      .leftMap[RpcError](RpcRequestFailed)
      .flatMap[RpcError, String] { resp ⇒
        val eitherResp = resp.body
          .leftMap[RpcError](RpcRequestErrored(resp.code, _))

        // Print just the first line of response
        Log.eitherT[F, RpcError].debug(s"TendermintRpc ${reqT.method.m} response code ${resp.code}") *>
          Log.eitherT[F, RpcError].trace(s"TendermintRpc ${reqT.method.m} full response: $eitherResp") *>
          EitherT.fromEither(eitherResp)
      }

  private def logPost(req: RpcRequest)(implicit log: Log[F]): EitherT[F, RpcError, Unit] =
    Log.eitherT[F, RpcError].debug(s"TendermintRpc POST method=${req.method}")

  private def logGet(path: String)(implicit log: Log[F]): EitherT[F, RpcError, Unit] =
    Log.eitherT[F, RpcError].debug(s"TendermintRpc GET path=$path")

  /**
   * Performs a Get request for the given path
   */
  private def get(path: String)(implicit log: Log[F]) =
    logGet(path) *> sendHandlingErrors(sttp.get(rpcUri.path(path)))

  /**
   * Performs a Post request, sending the given [[RpcRequest]]
   */
  private def post(req: RpcRequest)(implicit log: Log[F]) =
    logPost(req) *> sendHandlingErrors(sttp.post(rpcUri).body(req.toJsonString))

}

object TendermintRpc {

  /**
   * Runs a WorkerRpc with F effect, acquiring some resources for it
   *
   * @param sttpBackend Sttp Backend to be used to make RPC calls
   * @param hostName Hostname to query status from
   * @param port Port to query status from
   * @tparam F Concurrent effect
   * @return Worker RPC instance. Note that it should be stopped at some point, and can't be used after it's stopped
   */
  def make[F[_]: Sync: Log](
    hostName: String,
    port: Short
  )(implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing]): Resource[F, TendermintRpc[F]] =
    Resource.liftF {
      val rpc = new TendermintRpc[F](hostName, port)
      Log[F].info(s"TendermintRpc created, uri: ${rpc.rpcUri}") as rpc
    }
}
