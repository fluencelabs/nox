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

package fluence.ethclient

import cats.effect.concurrent.Deferred
import cats.effect.{ExitCode, IO, IOApp}
import org.web3j.abi.EventEncoder

import scala.concurrent.duration._

object EthClientApp extends IOApp {
  override def run(args: List[String]): IO[ExitCode] =
    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒
        for {
          _ ← IO(println("Launching w3j"))

          unsubscribe ← Deferred[IO, Either[Throwable, Unit]]

          version ← ethClient.clientVersion[IO]()
          _ ← IO(println(s"Client version: $version"))

          _ = ethClient
            .subscribeToLogsTopic[IO](
              "0xf93568cdc75b8849f4999bd3c8c6f931a14b258f",
              EventEncoder.buildEventSignature("NewSolver(bytes32)")
            )
            .map(log ⇒ println(s"Log message: $log"))
            .interruptWhen(unsubscribe)
            .drain
            .compile
            .drain
            .unsafeRunAsyncAndForget()

          _ ← IO(println(s"Subscribed"))

          _ ← IO.sleep(600.seconds)
          _ ← IO(println(s"Going to unsubscribe"))
          _ ← unsubscribe
        } yield ()
      }
      .map { _ ⇒
        println("okay that's all")
        ExitCode.Success
      }

}
