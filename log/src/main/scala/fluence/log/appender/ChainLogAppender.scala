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

package fluence.log.appender

import cats.Monad
import cats.data.Chain
import cats.effect.Sync
import cats.effect.concurrent.Ref
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.order._
import fluence.log.Log

import scala.language.higherKinds

/**
 * Attaches log messages to the tail of a [[Chain]], so that they could be printed in a batch later
 *
 * @param data Log Data
 * @tparam F Effect
 */
class ChainLogAppender[F[_]: Monad](private val data: Ref[F, Chain[Log.Msg]]) extends LogAppender[F] {
  override private[log] def appendMsg(msg: Log.Msg): F[Unit] =
    data.update(_.append(msg))

  /**
   * Make a string from all the logs
   *
   * @param level Log level to filter the messages
   * @return \n-glued string of all the logs
   */
  def mkStringF(level: Log.Level = Log.Trace)(): F[String] =
    data.get.map(_.iterator.filter(_.level >= level).mkString("\n"))

  /**
   * Reset the chain, and call a function on previously collected batch
   *
   * @param level Level to filter the messages with
   * @param onBatch Callback for the batch; it will be dropped after that
   * @return Unit after onBatch is handled
   */
  def handleBatch(
    onBatch: Iterator[Log.Msg] ⇒ F[Unit],
    level: Log.Level = Log.Trace
  ): F[Unit] =
    data.getAndSet(Chain.empty).map(_.iterator.filter(_.level >= level)) >>= onBatch
}

object ChainLogAppender {

  def apply[F[_]: Sync](): F[ChainLogAppender[F]] =
    Ref.of(Chain.empty[Log.Msg]).map(new ChainLogAppender[F](_))
}
