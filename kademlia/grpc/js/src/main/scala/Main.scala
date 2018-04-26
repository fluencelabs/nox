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

import fluence.crypto.signature.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.kad.grpc.client.KademliaJSClient
import fluence.kad.grpc.{KademliaGrpcService, KademliaNodeCodec}
import fluence.kad.protocol.Key
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.scalajs.js.annotation.{JSExport, JSExportTopLevel}

@JSExportTopLevel("Main")
object Main extends slogging.LazyLogging {

  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.DEBUG

  val algo: SignAlgo = Ecdsa.signAlgo
  import algo.checker

  implicit val codec = KademliaNodeCodec.pureCodec

  val host = "http://localhost:8090"

  val grpcService = KademliaGrpcService(host, true)
  val client = new KademliaJSClient(grpcService)

  @JSExport
  def logic(): Unit = {
    val keyP = algo.generateKeyPair.unsafe(None)
    val key = Key.fromPublicKey(keyP.publicKey).value.toOption.get
    val io = for {
      node ← client.ping()
      _ = logger.info("Ping node response: " + node)
      listOfNodes ← client.lookup(key, 2)
      _ = logger.info("Lookup nodes response: " + listOfNodes.mkString("\n"))
      key2 = listOfNodes.head.key
      listOfNodes2 ← client.lookupAway(key2, key, 2)
    } yield {
      logger.info("Lookup away nodes response: " + listOfNodes2.mkString("\n", "\n", "\n"))
    }

    io.attempt.unsafeRunAsync(res ⇒ logger.info("Result: " + res))
  }

  def main(args: Array[String]): Unit = {
    logic()
  }
}
