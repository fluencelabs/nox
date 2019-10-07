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

package fluence.worker.responder.repeat

import fluence.bp.tx.Tx
import fs2.concurrent.Topic

import scala.language.higherKinds

/**
 * Data about subscription.
 *
 * @param tx transaction that will be processed after each block
 * @param topic for publishing events to subscribers
 * @param subNumber number of subscriptions, the subscription should be deleted after subNumber become zero
 */
private[repeat] case class Subscription[F[_]](
  tx: Tx.Data,
  topic: Topic[F, Event],
  subNumber: Int
)
