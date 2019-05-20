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

package fluence.log

import cats.data.Chain
import cats.syntax.functor._
import cats.syntax.order._
import cats.effect.{Clock, Sync}
import cats.effect.concurrent.Ref

import scala.language.higherKinds

/**
 * Functional logger facade
 *
 * @param ctx Trace Context
 * @param data Log Data
 * @tparam F Effect
 */
class ChainLog[F[_]: Sync: Clock](override val ctx: Context, private val data: Ref[F, Chain[Log.Msg]]) extends Log[F] {

  /**
   * Provide a logger with modified context
   *
   * @param modContext Context modification
   * @param fn         Function to use the new logger
   * @tparam A Return type
   * @return What the inner function returns
   */
  override def scope[A](modContext: Context ⇒ Context)(fn: Log[F] ⇒ F[A]): F[A] =
    fn(new ChainLog(modContext(ctx), data))

  override protected def appendMsg(msg: Log.Msg): F[Unit] =
    data.update(_.append(msg))

  def mkStringF(level: Log.Level = ctx.loggingLevel): F[String] =
    data.get.map(_.iterator.filter(_.level >= level).mkString("\n"))
}

object ChainLog {

  implicit def forCtx[F[_]: Sync: Clock](implicit ctx: Context): ChainLog[F] =
    new ChainLog[F](ctx, Ref.unsafe(Chain.empty))

}
