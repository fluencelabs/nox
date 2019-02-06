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
import cats.effect.{Concurrent, Resource, Sync}
import cats.syntax.either._
import cats.syntax.functor._
import com.softwaremill.sttp._
import com.softwaremill.sttp.circe.asJson
import fluence.node.workers.WorkerParams
import cats.syntax.applicativeError._
import fluence.node.MakeResource
import fluence.node.workers.tendermint.status.StatusResponse
import io.circe.generic.semiauto._
import io.circe.{Encoder, Json}

import scala.language.higherKinds

/**
 * Provides a single concurrent endpoint to run RPC requests on Worker
 *
 * @param sinkRpc Tendermint's RPC requests endpoint
 * @param status Tendermint's status
 * @tparam F Concurrent effect
 */
case class TendermintRpc[F[_]] private (
  sinkRpc: fs2.Sink[F, TendermintRpc.Request],
  status: EitherT[F, Throwable, StatusResponse.WorkerTendermintInfo]
) {

  val broadcastTxCommit: fs2.Sink[F, String] =
    (s: fs2.Stream[F, String]) ⇒ s.map(TendermintRpc.broadcastTxCommit(_)) to sinkRpc

  /**
   * Make a single RPC call in a fire-and-forget manner.
   * Response is to be dropped, so you should take care of `id` in the request if you need to get it
   *
   * @param req The Tendermint request
   * @param F Concurrent effect
   */
  def callRpc(req: TendermintRpc.Request)(implicit F: Concurrent[F]): F[Unit] =
    fs2.Stream(req).to(sinkRpc).compile.drain

}

object TendermintRpc {
  private val requestEncoder: Encoder[Request] = deriveEncoder[Request]

  /**
   * Wrapper for Tendermint's RPC request
   *
   * @param method Method name
   * @param jsonrpc Version of the JSON RPC protocol
   * @param params Sequence of arguments for the method
   * @param id Nonce to track the results of the request with some other method
   */
  case class Request(method: String, jsonrpc: String = "2.0", params: Seq[Json], id: String = "") {
    def toJsonString: String = requestEncoder(this).noSpaces
  }

  /**
   * Builds a broadcast_tx_commit RPC request
   *
   * @param tx Transaction body
   * @param id Tracking ID, you may omit it
   * NOTE from Tendermint docs: it is not possible to send transactions to Tendermint during `Commit` - if your app tries to send a `/broadcast_tx` to Tendermint during Commit, it will deadlock.
   * TODO: ensure the above deadlock doesn't happen
   */
  def broadcastTxCommit(tx: String, id: String = ""): Request =
    Request(method = "broadcast_tx_commit", params = Json.fromString(tx) :: Nil, id = id)

  private def status[F[_]: Sync](
    uri: Uri
  )(implicit sttpBackend: SttpBackend[F, Nothing]): EitherT[F, Throwable, StatusResponse.WorkerTendermintInfo] =
    EitherT {
      sttp
        .get(uri)
        .response(asJson[StatusResponse])
        .send()
        .attempt
        // converting Either[Throwable, Response[Either[DeserializationError[circe.Error], WorkerResponse]]]
        // to Either[Throwable, WorkerResponse]
        .map(
          _.flatMap(
            _.body
              .leftMap(new Exception(_))
              .flatMap(_.leftMap(_.error))
          )
        )
    }.map(_.result)

  /**
   * Runs a WorkerRpc with F effect, acquiring some resources for it
   *
   * @param sttpBackend Sttp Backend to be used to make RPC calls
   * @param hostName Hostname to query status from
   * @param port Port to query status from
   * @tparam F Concurrent effect
   * @return Worker RPC instance. Note that it should be stopped at some point, and can't be used after it's stopped
   */
  def make[F[_]: Concurrent](
    hostName: String,
    port: Short
  )(implicit sttpBackend: SttpBackend[F, Nothing]): Resource[F, TendermintRpc[F]] = {
    def rpcUri(hostName: String, path: String = ""): Uri =
      uri"http://$hostName:$port/$path"

    for {
      queue ← Resource.liftF(fs2.concurrent.Queue.unbounded[F, TendermintRpc.Request])

      _ ← MakeResource.concurrentStream(
        queue.dequeue.evalMap(
          req ⇒
            sttpBackend
              .send(
                sttp
                  .post(rpcUri(hostName))
                  .body(req.toJsonString)
              )
              .map(_.isSuccess)
        ),
        name = s"tendermintRpc-${params.appId}"
      )
    } yield
      new TendermintRpc[F](
        queue.enqueue,
        status[F](rpcUri(hostName, "status"))
      )
  }
}
