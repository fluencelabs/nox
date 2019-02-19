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

package fluence.node.workers.tendermint.rpc

import cats.data.EitherT
import cats.effect.{Resource, Sync}
import cats.syntax.either._
import cats.syntax.functor._
import com.softwaremill.sttp._
import cats.syntax.applicativeError._
import fluence.node.workers.status.{HttpCheckFailed, HttpCheckStatus, HttpStatus}
import io.circe.parser.decode
import io.circe.Json

import scala.language.higherKinds

/**
 * Provides a single concurrent endpoint to run RPC requests on Worker
 *
 * @tparam F Concurrent effect
 */
case class TendermintRpc[F[_]: Sync](
  get: String ⇒ EitherT[F, RpcError, String],
  post: RpcRequest ⇒ EitherT[F, RpcError, String]
) {

  val status: EitherT[F, RpcError, TendermintStatus] =
    get("status")
      .map(decode[StatusResponse])
      .subflatMap[RpcError, TendermintStatus](
        _.map(_.result).leftMap(RpcBodyMalformed)
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
  def broadcastTxCommit(tx: String, id: String = ""): EitherT[F, RpcError, String] =
    post(RpcRequest(method = "broadcast_tx_commit", params = Json.fromString(tx) :: Nil, id = id))

  /**
   * Performs http status check, lifting result to [[HttpStatus]] data type
   */
  def httpStatus: F[HttpStatus[TendermintStatus]] =
    status.value.map {
      case Right(resp) ⇒ HttpCheckStatus(resp)
      case Left(err) ⇒ HttpCheckFailed(err)
    }
}

object TendermintRpc {

  private def sendHandlingErrors[F[_]: Sync](
    reqT: RequestT[Id, String, Nothing]
  )(implicit sttpBackend: SttpBackend[F, Nothing]): EitherT[F, RpcError, String] =
    reqT
      .send()
      .attemptT
      .leftMap[RpcError](RpcRequestFailed)
      .subflatMap[RpcError, String](
        resp ⇒
          resp.body
            .leftMap[RpcError](RpcRequestErrored(resp.code, _))
      )

  private def get[F[_]: Sync](uri: Uri)(implicit sttpBackend: SttpBackend[F, Nothing]): EitherT[F, RpcError, String] =
    sendHandlingErrors(
      sttp.get(uri)
    )

  private def post[F[_]: Sync](uri: Uri, req: RpcRequest)(
    implicit sttpBackend: SttpBackend[F, Nothing]
  ): EitherT[F, RpcError, String] =
    sendHandlingErrors(
      sttp
        .post(uri)
        .body(req.toJsonString)
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
  )(implicit sttpBackend: SttpBackend[F, Nothing]): Resource[F, TendermintRpc[F]] = {
    def rpcUri(hostName: String, path: String = ""): Uri =
      uri"http://$hostName:$port/$path"

    Resource.pure(
      new TendermintRpc[F](
        path ⇒ get[F](rpcUri(hostName, path)),
        req ⇒ post[F](rpcUri(hostName), req)
      )
    )
  }
}
