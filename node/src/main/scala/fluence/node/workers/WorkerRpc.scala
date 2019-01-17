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
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.apply._
import com.softwaremill.sttp._
import io.circe.{Encoder, Json}
import io.circe.generic.semiauto._

import scala.language.higherKinds

case class WorkerRpc[F[_]] private (
  sinkRpc: fs2.Sink[F, WorkerRpc.Request],
  private[workers] val stop: F[Unit]
) {

  def callRpc(req: WorkerRpc.Request)(implicit F: Concurrent[F]): F[Unit] =
    fs2.Stream(req).to(sinkRpc).compile.drain

}

object WorkerRpc {
  private val requestEncoder: Encoder[Request] = deriveEncoder[Request]
  case class Request(method: String, jsonrpc: String = "2.0", params: Seq[Json], id: String = "") {
    def toJsonString: String = requestEncoder(this).noSpaces
  }

  def broadcastTxCommit(tx: String, id: String = ""): Request =
    Request(method = "broadcast_tx_commit", params = Json.fromString(tx) :: Nil, id = id)

  private def toSomePipe[F[_], T]: fs2.Pipe[F, T, Option[T]] =
    _.map(Some(_))

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
                    .post(uri"http://${params.clusterData.rpcHost}:${params.rpcPort}/")
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
