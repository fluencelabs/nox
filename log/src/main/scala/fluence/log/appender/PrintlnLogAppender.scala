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

import cats.syntax.applicative._
import cats.effect.Sync
import fluence.log.Log

import scala.language.higherKinds

/**
 * Functional logger facade
 *
 * @tparam F Effect
 */
class PrintlnLogAppender[F[_]: Sync] extends LogAppender[F] {

  override private[log] def appendMsg(msg: Log.Msg): F[Unit] =
    Sync[F].delay(println(msg))

}

object PrintlnLogAppender {

  def apply[F[_]: Sync](): F[PrintlnLogAppender[F]] =
    new PrintlnLogAppender[F].pure[F]

}
