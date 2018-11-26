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

package fluence.statemachine
import cats.data.EitherT
import cats.effect.{ContextShift, IO}
import cats.effect.concurrent.MVar
import com.github.jtendermint.jabci.socket.TSocket
import com.github.jtendermint.jabci.types.Request.ValueCase.{CHECK_TX, DELIVER_TX}
import fluence.statemachine.config.StateMachineConfig
import fluence.statemachine.contract.ClientRegistry
import fluence.statemachine.error.{ConfigLoadingError, StateMachineError}
import fluence.statemachine.state._
import fluence.statemachine.tx.{TxParser, TxProcessor, TxStateDependentChecker}
import slogging.{LazyLogging, LogLevel, LoggerConfig}

import scala.concurrent.ExecutionContext.Implicits.global

object StateMachine extends LazyLogging {
  implicit private val cs: ContextShift[IO] = IO.contextShift(global)

  private def buildAbciHandler(
    config: StateMachineConfig,
    invoker: Invoker[IO]
  ): EitherT[IO, StateMachineError, AbciHandler] = {
    logger.info("Building State Machine ABCI handler")
    for {
      //      _ = Metrics.resetCollectors()
      initialState <- EitherT.right(MVar[IO].of(TendermintState.initial))
      stateHolder = new TendermintStateHolder[IO](initialState)
      mutableConsensusState = new MutableStateTree(stateHolder)

      queryProcessor = new QueryProcessor(stateHolder)

      txParser = new TxParser[IO](new ClientRegistry())
      checkTxStateChecker = new TxStateDependentChecker[IO](CHECK_TX, stateHolder.mempoolState)
      deliverTxStateChecker = new TxStateDependentChecker(DELIVER_TX, mutableConsensusState.getRoot)

      txProcessor = new TxProcessor(mutableConsensusState, invoker, config)

      committer = new Committer[IO](stateHolder, () => invoker.stateHash())

      abciHandler = new AbciHandler(
        committer,
        queryProcessor,
        txParser,
        checkTxStateChecker,
        deliverTxStateChecker,
        txProcessor
      )
    } yield abciHandler
  }

  def listen(invoker: Invoker[IO], port: Int) =
    for {
      config <- loadConfig()
      _ = configureLogLevel(config.logLevel)

      abciHandler <- buildAbciHandler(config, invoker)

      _ = logger.info("Starting State Machine ABCI handler")
      socket = new TSocket
      _ = socket.registerListener(abciHandler)

      socketThread = new Thread(() => socket.start(port))
      _ = socketThread.setName("Socket")
      _ = socketThread.start()
      _ = socketThread.join()
    } yield {}

  /**
   * Loads State machine config using `pureconfig` Scala config loading mechanism.
   */
  private def loadConfig(): EitherT[IO, StateMachineError, StateMachineConfig] =
    EitherT
      .fromEither[IO](pureconfig.loadConfig[StateMachineConfig])
      .leftMap(
        e => ConfigLoadingError("Unable to read StateMachineConfig: " + e.toList)
      )

  /**
   * Configures `slogging` log level.
   *
   * @param logLevel level of logging
   */
  private def configureLogLevel(logLevel: String): Unit =
    LoggerConfig.level = logLevel.toUpperCase match {
      case "OFF" => LogLevel.OFF
      case "ERROR" => LogLevel.ERROR
      case "WARN" => LogLevel.WARN
      case "INFO" => LogLevel.INFO
      case "DEBUG" => LogLevel.DEBUG
      case "TRACE" => LogLevel.TRACE
      case _ => LogLevel.INFO
    }

}
