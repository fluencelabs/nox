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

package fluence.node.workers.subscription

import cats.effect.concurrent.Deferred
import fluence.statemachine.data.Tx

import scala.language.higherKinds

/**
 * Unit to manage subscriptions.
 *
 * @param id transaction id: sessionId/nonce
 * @param promise a promise that will be completed after response will be received
 * @param tries how many times we already query a state machine
 */
case class ResponsePromise[F[_]](id: Tx.Head, promise: Deferred[F, TendermintQueryResponse], tries: Int = 0) {
  def complete(response: TendermintQueryResponse): F[Unit] = promise.complete(response)
}
