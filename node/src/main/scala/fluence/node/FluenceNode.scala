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

  override def run(args: List[String]): IO[ExitCode] =
    DockerIO
      .run[IO]("-p 9393:80 nginx")
      .through(
        // Check that container is running every 3 seconds
        DockerIO.check[IO](3.seconds)
      )
      .evalMap[IO, (FiniteDuration, Either[String, Unit])] {
        case (d, true) ⇒
          // As container is running, perform a custom healthcheck: request a HTTP endpoint inside the container
          sttp
            .get(uri"http://localhost:9393")
            .send()
            .attempt
            .map(_.left.map(_.getMessage).map(_ ⇒ ()))
            .map(d → _)

        case (d, false) ⇒ IO.pure(d → Left("Container is not running"))
      }
      .sliding(5)
      .evalTap[IO] {
        case q if q.count(_._2.isLeft) > 3 ⇒
          // Stop the stream, as there's too many failing healthchecks
          IO.raiseError(new RuntimeException("Too many failures"))
        case _ ⇒ IO.unit
      }
      .compile
      .last
      .map { last ⇒
        // Actually, this code will never run, as `.last` above could ever return only if the stream is interrupted with error
        println(s"Finally: $last")
        ExitCode.Success
      }
}
