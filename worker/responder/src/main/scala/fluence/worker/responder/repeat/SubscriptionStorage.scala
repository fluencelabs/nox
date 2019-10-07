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

import cats.Monad
import cats.effect.Sync
import cats.effect.concurrent.Ref
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.bp.tx.Tx
import fluence.log.Log
import fluence.worker.responder.resp.AwaitedResponse

import scala.language.higherKinds

class SubscriptionStorage[F[_]: Monad](subscriptions: Ref[F, Map[SubscriptionKey, SubscriptionStorage.Value[F]]]) {

  import SubscriptionStorage.Value

  def getSubscriptions: F[Map[SubscriptionKey, Value[F]]] = subscriptions.get

  /**
   * Add stream to a subscription. Subscription must be marked as being used via [[addSubscription]].
   *
   * @return false if subscription with such key already exists in the map, stating whether operation was successful
   */
  def addStream(key: SubscriptionKey, stream: fs2.Stream[F, AwaitedResponse.OrError])(implicit log: Log[F]): F[Unit] =
    for {
      subExists <- subscriptions.modify { subs =>
        if (subs.contains(key)) {
          (subs.updated(key, Some(stream)), true)
        } else {
          (subs, false)
        }
      }
      _ <- if (!subExists) log.warn("Unexpected. There is no subscription for a created stream.") else ().pure[F]
    } yield ()

  /**
   * Mark subscription as being used, to mitigate data race possibility
   *
   * @return false if subscription with such key already exists in the map, stating whether operation was successful
   */
  def addSubscription(key: SubscriptionKey, tx: Tx.Data)(implicit log: Log[F]): F[Boolean] =
    for {
      subExists <- subscriptions.modify { subs =>
        if (subs.contains(key)) {
          (subs, true)
        } else {
          (subs + (key -> None), false)
        }
      }
    } yield !subExists

  def deleteSubscription(key: SubscriptionKey)(implicit log: Log[F]): F[Unit] =
    subscriptions.update(_ - key)
}

object SubscriptionStorage {

  type Value[F[_]] = Option[fs2.Stream[F, AwaitedResponse.OrError]]

  def apply[F[_]: Sync](): F[SubscriptionStorage[F]] =
    for {
      storage <- Ref.of(Map.empty[SubscriptionKey, Value[F]])
    } yield new SubscriptionStorage[F](storage)

}
