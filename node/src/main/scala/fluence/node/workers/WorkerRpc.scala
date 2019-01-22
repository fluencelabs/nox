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

package fluence.node.workers
import cats.effect.Concurrent
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.softwaremill.sttp._
import io.circe.generic.semiauto._
import io.circe.{Encoder, Json}

import scala.language.higherKinds

/**
 * Provides a single concurrent endpoint to run RPC requests on Worker
 *
 * @param sinkRpc Tendermint's RPC requests endpoint
 * @param stop A callback that releases the acquired resources, namely it stops the RPC stream
 * @tparam F Concurrent effect
 */
case class WorkerRpc[F[_]] private (
  sinkRpc: fs2.Sink[F, WorkerRpc.Request],
  private[workers] val stop: F[Unit]
) {

  /**
   * Make a single RPC call in a fire-and-forget manner.
   * Response is to be dropped, so you should take care of `id` in the request if you need to get it
   *
   * @param req The Tendermint request
   * @param F Concurrent effect
   */
  def callRpc(req: WorkerRpc.Request)(implicit F: Concurrent[F]): F[Unit] =
    fs2.Stream(req).to(sinkRpc).compile.drain

}

object WorkerRpc {
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

  private def toSomePipe[F[_], T]: fs2.Pipe[F, T, Option[T]] =
    _.map(Some(_))

  /**
   * Runs a WorkerRpc with F effect, acquiring some resources for it
   *
   * @param params Worker params to get Worker URI from
   * @param sttpBackend Sttp Backend to be used to make RPC calls
   * @tparam F Concurrent effect
   * @return Worker RPC instance. Note that it should be stopped at some point, and can't be used after it's stopped
   */
  def apply[F[_]: Concurrent](params: WorkerParams)(implicit sttpBackend: SttpBackend[F, Nothing]): F[WorkerRpc[F]] =
    for {
      queue ← fs2.concurrent.Queue.noneTerminated[F, WorkerRpc.Request]
      fiber ← Concurrent[F].start(
        queue.dequeue
          .evalMap(
            req ⇒
              sttpBackend
                .send(
                  sttp
                    .post(uri"http://${params.currentWorker.ip.getHostAddress}:${params.currentWorker.rpcPort}/")
                    .body(req.toJsonString)
                )
                .map(_.isSuccess)
          )
          .drain
          .compile
          .drain
      )

      enqueue = queue.enqueue
      stop = fs2.Stream(None).to(enqueue).compile.drain *> fiber.join
      callRpc = toSomePipe[F, WorkerRpc.Request].andThen(_ to enqueue)

    } yield new WorkerRpc[F](callRpc, stop)
}
