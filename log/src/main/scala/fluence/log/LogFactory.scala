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

import cats.Monad
import cats.effect.{Clock, Sync}
import cats.syntax.functor._
import fluence.log.appender.{ChainLogAppender, LogAppender, PrintlnLogAppender}

import scala.language.higherKinds

abstract class LogFactory[F[_]: Monad: Clock](defaultLoggingLevel: Log.Level) {
  self ⇒

  type Appender <: LogAppender[F]

  def newAppender(): F[Appender]

  def forCtx(implicit ctx: Context): F[Log.Aux[F, Appender]] =
    newAppender().map(
      a ⇒
        new Log(ctx) {
          override type Appender = self.Appender
          override val appender: self.Appender = a
      }
    )

  def init(k: String, v: String = "", level: Log.Level = defaultLoggingLevel): F[Log.Aux[F, Appender]] =
    forCtx(Context.init(k, v, level))

}

object LogFactory {
  type Aux[F[_], A] = LogFactory[F] { type Appender = A }

  def apply[F[_]](implicit lf: LogFactory[F]): LogFactory[F] = lf

  def forChains[F[_]: Clock: Sync](level: Log.Level = Log.Info): LogFactory.Aux[F, ChainLogAppender[F]] =
    new LogFactory[F](level) {
      override type Appender = ChainLogAppender[F]

      override def newAppender(): F[ChainLogAppender[F]] =
        ChainLogAppender[F]()
    }

  def forPrintln[F[_]: Clock: Sync](level: Log.Level = Log.Info): LogFactory.Aux[F, PrintlnLogAppender[F]] =
    new LogFactory[F](level) {
      override type Appender = PrintlnLogAppender[F]

      override def newAppender(): F[PrintlnLogAppender[F]] =
        PrintlnLogAppender[F]()
    }
}
