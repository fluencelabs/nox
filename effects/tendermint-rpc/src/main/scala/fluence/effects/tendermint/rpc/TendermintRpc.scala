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
import cats.syntax.either._
import com.softwaremill.sttp._
import cats.syntax.applicativeError._
import fluence.effects.tendermint.rpc.response.{Response, TendermintStatus}
import io.circe.parser.decode
import io.circe.Json

import scala.language.higherKinds

/**
 * Provides a single concurrent endpoint to run RPC requests on Worker
 *
 * @param get Perform a Get request for the given path
 * @param post Perform a Post request, sending the given [[RpcRequest]]
 * @tparam F Http requests effect
 */
case class TendermintRpc[F[_]](
  get: String ⇒ EitherT[F, RpcError, String],
  post: RpcRequest ⇒ EitherT[F, RpcError, String]
) extends slogging.LazyLogging {

  /** Get status as string */
  val status: EitherT[F, RpcError, String] =
    get("status")

  /** Get status, parse it to [[TendermintStatus]] */
  def statusParsed(implicit F: Functor[F]): EitherT[F, RpcError, TendermintStatus] =
    status
      .map(decode[Response[TendermintStatus]])
      .subflatMap[RpcError, TendermintStatus](
        _.map(_.result).leftMap(RpcBodyMalformed)
      )

  def block(height: Long, id: String = "dontcare"): EitherT[F, RpcError, String] =
    post(
      RpcRequest(
        method = "block",
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
  def broadcastTxSync(tx: String, id: String): EitherT[F, RpcError, String] =
    post(
      RpcRequest(
        method = "broadcast_tx_sync",
        params = Json.fromString(java.util.Base64.getEncoder.encodeToString(tx.getBytes)) :: Nil,
        id = id
      )
    )

  def unsafeDialPeers(peers: Seq[String], persistent: Boolean, id: String = "dontcare"): EitherT[F, RpcError, String] =
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
}

object TendermintRpc extends slogging.LazyLogging {

  /** Perform the request, and lift the errors to EitherT */
  private def sendHandlingErrors[F[_]: Sync](
    reqT: RequestT[Id, String, Nothing]
  )(implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing]): EitherT[F, RpcError, String] =
    EitherT
      .pure(logger.debug(s"TendermintRpc request $reqT"))
      .flatMap(
        _ =>
          reqT
            .send()
            .leftMap[RpcError](RpcRequestFailed)
            .subflatMap[RpcError, String] { resp ⇒
              val eitherResp = resp.body
                .leftMap[RpcError](RpcRequestErrored(resp.code, _))

              logger.debug(s"TendermintRpc response(${resp.code}): $eitherResp")
              eitherResp
          }
      )

  /**
   * Runs a WorkerRpc with F effect, acquiring some resources for it
   *
   * @param sttpBackend Sttp Backend to be used to make RPC calls
   * @param hostName Hostname to query status from
   * @param port Port to query status from
   * @tparam F Concurrent effect
   * @return Worker RPC instance. Note that it should be stopped at some point, and can't be used after it's stopped
   */
  def make[F[_]: Sync](
    hostName: String,
    port: Short
  )(implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing]): Resource[F, TendermintRpc[F]] = {
    def rpcUri(hostName: String, path: String = ""): Uri =
      uri"http://$hostName:$port/$path"

    Resource.pure(
      new TendermintRpc[F](
        path ⇒
          sendHandlingErrors(
            sttp.get(rpcUri(hostName, path))
        ),
        req ⇒
          sendHandlingErrors(
            sttp
              .post(rpcUri(hostName))
              .body(req.toJsonString)
        )
      )
    )
  }
}
