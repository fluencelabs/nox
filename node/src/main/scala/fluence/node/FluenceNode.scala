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

import cats.effect.{ExitCode, IO, IOApp}
import cats.syntax.apply._

import com.softwaremill.sttp._
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend

import scala.concurrent.ExecutionContext

object FluenceNode extends IOApp {
  private implicit val sttpBackend: SttpBackend[IO, Nothing] = AsyncHttpClientCatsBackend[IO]()

  slogging.LoggerConfig.level = slogging.LogLevel.INFO
  slogging.LoggerConfig.factory = slogging.PrintLoggerFactory

  val lines: fs2.Stream[IO, String] =
    fs2.io
      .stdin[IO](5, ExecutionContext.global)
      .through(fs2.text.utf8Decode)
      .through(fs2.text.lines[IO])
      .map(_.trim)

  private val RunR = "^run ([0-9]{3,5})$".r

  def handleCli(pool: SolversPool[IO]): IO[ExitCode] =
    lines
      .evalMap[IO, Option[ExitCode]] {
        case "stop" ⇒
          IO(println(s"Going to stop...")) *> pool.stopAll[IO.Par].map(_ ⇒ Some(ExitCode.Success))

        case "health" ⇒
          for {
            hs ← pool.healths[IO.Par]
            _ ← IO(println("Last health reports of solvers:\n" + hs.mkString("\n")))
          } yield None

        case RunR(port) ⇒
          for {
            _ <- IO(println(s"Going to run on $port"))
            _ ← pool.run(SolverParams(port.toInt))
          } yield None

        case unknown ⇒
          IO(println(s"Unknown command: $unknown")) *> IO.pure(None)
      }
      .evalTap[IO] {
        case None ⇒ IO(println("Please input the command"))
        case Some(_) ⇒ IO.unit
      }
      .unNone
      .head
      .compile
      .lastOrError

  override def run(args: List[String]): IO[ExitCode] =
    for {
      pool ← SolversPool[IO]
      _ = println("Pool Received, ready to run solvers. Please input the command")
      code ← handleCli(pool)
    } yield {
      println(Console.GREEN + s"Exit with $code" + Console.RESET)
      code
    }

}
