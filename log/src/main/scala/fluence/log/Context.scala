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

import scala.language.higherKinds
import scala.language.implicitConversions

/**
 * Context data, should be used in [[Log]] or any other place where it's useful to be captured
 *
 * @param data Context data
 */
case class Context(data: Map[String, String] = Map.empty, loggingLevel: Log.Level) {
  def scope(kv: (String, String)*): Context = copy(data = data ++ kv)

  def level(l: Log.Level): Context = copy(loggingLevel = l)

  override def toString: String =
    data.toSeq
      .map(kv ⇒ s"${Console.YELLOW}${kv._1}${Console.RESET}:${Console.WHITE}${kv._2}${Console.RESET}")
      .mkString(" ")
}

object Context {

  def init(kv: (String, String)*)(level: Log.Level = Log.Info): Context =
    Context(kv.toMap, level)

  implicit def fromLog[F[_]: Log]: Context =
    Log[F].ctx
}
