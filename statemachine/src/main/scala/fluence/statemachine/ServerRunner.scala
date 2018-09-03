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

package fluence.statemachine

import com.github.jtendermint.jabci.socket.TSocket
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging._

/**
 * Main class for the State machine.
 *
 * Launches JVM ABCI Server by instantiating jTendermint's `TSocket` registering `ABCIHandler` as handler.
 * jTendermint implements RPC layer, provides dedicated threads (`Consensus`, `Mempool` and `Query` thread
 * according to Tendermint specification) and sends ABCI requests to `ABCIHandler`.
 */
object ServerRunner extends LazyLogging {
  val DefaultABCIPoint: Int = 46658

  def main(args: Array[String]): Unit = {
    val port = if (args.length > 0) args(0).toInt else DefaultABCIPoint
    ServerRunner.start(port)
  }

  def start(port: Int): Unit = {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(true, false, true)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.INFO

    logger.info("starting State Machine")
    val socket = new TSocket

    val abciHandler = new ABCIHandler
    socket.registerListener(abciHandler)

    val socketThread = new Thread(() => socket.start(port))
    socketThread.setName("Socket")
    socketThread.start()
    socketThread.join()
  }
}
