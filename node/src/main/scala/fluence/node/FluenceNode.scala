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

package fluence.node
import java.util.concurrent.ExecutorService

import cats.effect.{ExitCode, IO, IOApp}

import scala.concurrent.duration._
import com.softwaremill.sttp._
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend

import scala.concurrent.ExecutionContext
import scala.util.Try

object FluenceNode extends IOApp {
  private implicit val sttpBackend: SttpBackend[IO, Nothing] = AsyncHttpClientCatsBackend[IO]()

  slogging.LoggerConfig.level = slogging.LogLevel.DEBUG
  slogging.LoggerConfig.factory = slogging.PrintLoggerFactory

  val fs2ReadPort: fs2.Stream[IO, Int] =
    fs2.io
      .stdin[IO](5, ExecutionContext.global)
      .through(fs2.text.utf8Decode)
      .through(fs2.text.lines[IO])
      .map(s ⇒ Try(s.toInt).toOption)
      .collect {
        case Some(port) ⇒ port
      }

  override def run(args: List[String]): IO[ExitCode] =
    for {
      pool ← SolversPool[IO]
      _ = println("port?")
      port ← fs2ReadPort.head.compile.lastOrError
      _ ← pool.run(Solver.Params(port))
      _ = println("anything numeric?")
      _ ← fs2ReadPort.head.compile.last
      _ ← pool.stopAll
    } yield ExitCode.Success

}
