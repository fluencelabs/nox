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

import cats.Monad
import cats.data.EitherT
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.log.Log
import cats.syntax.flatMap._
import cats.syntax.either._
import io.circe.generic.semiauto._
import io.circe._
import cats.syntax.functor._
import fluence.node.workers.subscription.{OkResponse, PendingResponse, RpcErrorResponse, TimedOutResponse}
import fluence.node.workers.websocket.WebsocketRequests.{QueryRequest, TxWaitRequest, WebsocketRequest}
import io.circe.generic.extras.{Configuration, ConfiguredJsonCodec}
import io.circe.{Decoder, Encoder}
import io.circe.generic.extras.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.parser.parse
import io.circe.syntax._

import scala.language.higherKinds

object TestCase extends App {
  val req: WebsocketRequest = QueryRequest("path", Some("data"), None, requestId = "requestId")
  val req2: WebsocketRequest = TxWaitRequest("requestId2", None, "data2")
  println(req.asJson.spaces4)
  println(req2.asJson.spaces4)
}
