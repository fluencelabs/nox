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

import cats.effect.IO
import cats.instances.try_._
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.kad.grpc.client.KademliaJSClient
import fluence.kad.grpc.{KademliaGrpcService, KademliaNodeCodec}
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.scalajs.js.annotation.JSExport

@JSExport
object Main extends slogging.LazyLogging {

  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.DEBUG

  val algo: SignAlgo = Ecdsa.signAlgo
  import algo.checker

  implicit val codec = KademliaNodeCodec.codec[IO]

  val host = "http://localhost:8080"

  val grpcService = KademliaGrpcService(host, false)
  val client = new KademliaJSClient(grpcService)

  @JSExport
  def someCoolMethod(): Unit = {
    println("Hello world!")
    val io = for {
      node ← client.ping()
      _ = logger.info("Ping node response: " + node)
      listOfNodes ← client.lookup(node.key, 2)
      _ = logger.info("Lookup nodes response: " + listOfNodes.mkString("\n"))
      key2 = listOfNodes.head.key
      listOfNodes2 ← client.lookupAway(key2, node.key, 2)
    } yield {
      logger.info("Lookup away nodes response: " + listOfNodes2.mkString("\n", "\n", "\n"))
    }

    io.attempt.unsafeRunAsync(_ ⇒ ())
  }

  def main(args: Array[String]): Unit = {
    someCoolMethod()
  }
}
