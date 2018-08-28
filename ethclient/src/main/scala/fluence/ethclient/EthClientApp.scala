/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.ethclient

import cats.effect.{ExitCode, IO, IOApp}
import org.web3j.abi.EventEncoder

import scala.concurrent.duration._
import scala.concurrent.ExecutionContext.Implicits.global

object EthClientApp extends IOApp {

  override def run(args: List[String]): IO[ExitCode] =
    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒
        for {
          _ ← IO(println("Launching w3j"))

          version ← ethClient.clientVersion[IO]()
          _ ← IO(println(s"Client version: $version"))

          unsubscribe ← ethClient.subscribeToLogsTopic[IO, IO](
            "0xB8c77482e45F1F44dE1745F52C74426C631bDD52",
            EventEncoder.buildEventSignature("Transfer(address,address,uint)"),
            log ⇒ IO(println(s"Log message: $log"))
          )
          _ ← IO(println(s"Subscribed"))

          _ ← IO.sleep(60.seconds)
          _ ← IO(println(s"Going to unsubscribe"))
          _ ← unsubscribe
        } yield ()

      }
      .map { _ ⇒
        println("okay that's all")
        ExitCode.Success
      }

}
