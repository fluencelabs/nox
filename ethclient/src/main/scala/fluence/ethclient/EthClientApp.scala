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

import cats.Parallel
import cats.effect.concurrent.Deferred
import cats.effect.{ExitCode, IO, IOApp}
import org.web3j.abi.EventEncoder

import scala.concurrent.duration._

object EthClientApp extends IOApp {
  // EthClientApp is to play during development only
  override def run(args: List[String]): IO[ExitCode] =
    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒
        val par = Parallel[IO, IO.Par]

        (for {
          _ ← IO(println("Launching w3j"))

          isSyncing ← ethClient.isSyncing[IO].map(_.isSyncing)
          _ ← IO(println(s"isSyncing: $isSyncing"))

          version ← ethClient.clientVersion[IO]()
          _ = println(s"Client version: $version")

          _ ← ethClient.blockStream[IO]().map(println).compile.drain

          unsubscribe ← Deferred[IO, Either[Throwable, Unit]]

          _ ← par sequential par.apply.product(
            // Subscription stream
            par parallel ethClient
              .subscribeToLogsTopic[IO](
                "0x9995882876ae612bfd829498ccd73dd962ec950a",
                Network.NEWNODE_EVENT
              )
              .map(log ⇒ println(s"Log message: $log"))
              .interruptWhen(unsubscribe)
              .drain // drop the results, so that demand on events is always provided
              .onFinalize(IO(println("Subscription finalized")))
              .compile // Compile to a runnable, in terms of effect IO
              .drain, // Switch to IO[Unit]
            // Delayed unsubscribe
            par.parallel(for {
              _ ← IO.sleep(60.seconds)
              _ = println("Going to unsubscribe")
              _ ← unsubscribe.complete(Right(()))
            } yield ())
          )
        } yield ()).attempt
      }
      .map {
        case Right(_) ⇒
          println("okay that's all")
          ExitCode.Success

        case Left(err) ⇒
          println("hell, doesn't work: " + err)
          err.printStackTrace()
          ExitCode.Error
      }

}
