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

import scala.concurrent.duration._
import com.softwaremill.sttp._
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend

object FluenceNode extends IOApp {
  private implicit val sttpBackend: SttpBackend[IO, Nothing] = AsyncHttpClientCatsBackend[IO]()

  slogging.LoggerConfig.level = slogging.LogLevel.DEBUG
  slogging.LoggerConfig.factory = slogging.PrintLoggerFactory

  override def run(args: List[String]): IO[ExitCode] =
    (
      for {
        solver ← fs2.Stream.eval(Solver.run[IO](9393))
        _ = println(solver)
        fiber ← fs2.Stream.supervise(solver.fiber.join)
        _ = println("Supervising")
        _ ← fs2.Stream.sleep(40.seconds)
        _ = println("Woke up")
        _ ← fs2.Stream.eval_(solver.stop)
        health ← fs2.Stream.eval(solver.lastHealthCheck)
        _ ← fs2.Stream.eval_(fiber.join)
      } yield health
    ).compile.last.map { last ⇒
      println(s"Finally: $last")
      ExitCode.Success
    }
}
