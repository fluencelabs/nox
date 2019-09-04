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

package fluence.statemachine.abci

import cats.Applicative
import cats.data.{EitherT, NonEmptyList}
import cats.effect.concurrent.Deferred
import cats.effect.syntax.effect._
import cats.effect.{Effect, Resource, Sync}
import cats.syntax.applicative._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.github.jtendermint.jabci.api._
import com.github.jtendermint.jabci.socket.TSocket
import com.github.jtendermint.jabci.types._
import com.google.protobuf.ByteString
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.query.QueryResponse
import fluence.statemachine.api.signals.DropPeer
import fluence.statemachine.api.tx.{Tx, TxResponse}
import fluence.statemachine.api.{StateHash, StateMachineStatus}
import fluence.statemachine.control.signals.ControlSignals
import fluence.statemachine.error.StateMachineError
import fluence.statemachine.state.{StateMachineVm, StateService}
import fluence.statemachine.vm.WasmVmOperationInvoker

import scala.language.higherKinds
import scala.util.Try

class AbciHandler[F[_]: Effect: LogFactory](
  service: StateService[F],
  controlSignals: ControlSignals[F]
) extends ICheckTx with IDeliverTx with ICommit with IQuery with IEndBlock with IBeginBlock {

  override def requestBeginBlock(
    req: RequestBeginBlock
  ): ResponseBeginBlock = ResponseBeginBlock.newBuilder().build()

  private def handleTxResponse(tx: Array[Byte], resp: TxResponse)(implicit log: Log[F]): F[Unit] = {
    val TxResponse(code, info, height) = resp

    log.info(
      Tx.splitTx(tx)
        .fold(
          s"${height.fold("")(h => s"height $h")} can't parse head from tx ${Try(new String(tx.take(100)))}"
        ) {
          case (head, _) =>
            s"tx.head: $head -> $code ${info.replace('\n', ' ')} ${height.fold("")(h => s"height $h")}"
        }
    )
  }

  override def requestCheckTx(
    req: RequestCheckTx
  ): ResponseCheckTx = {
    implicit val log: Log[F] = LogFactory[F].init("abci", "requestCheckTx").toIO.unsafeRunSync()

    val tx = req.getTx.toByteArray

    service
      .checkTx(tx)
      .toIO
      .flatMap {
        case resp @ TxResponse(code, info, _) ⇒
          handleTxResponse(tx, resp).toIO
            .map(
              _ =>
                ResponseCheckTx.newBuilder
                  .setCode(code.id)
                  .setInfo(info)
                  // TODO where it goes?
                  .setData(ByteString.copyFromUtf8(info))
                  .build
            )
      }
      .unsafeRunSync()
  }

  override def receivedDeliverTx(
    req: RequestDeliverTx
  ): ResponseDeliverTx = {
    implicit val log: Log[F] = LogFactory[F].init("abci", "requestDeliverTx").toIO.unsafeRunSync()

    val tx = req.getTx.toByteArray

    service
      .deliverTx(tx)
      .toIO
      .flatMap {
        case resp @ TxResponse(code, info, _) ⇒
          handleTxResponse(tx, resp).toIO
            .map(
              _ =>
                ResponseDeliverTx.newBuilder
                  .setCode(code.id)
                  .setInfo(info)
                  // TODO where it goes?
                  .setData(ByteString.copyFromUtf8(info))
                  .build
            )
      }
      .unsafeRunSync()
  }

  override def requestCommit(
    requestCommit: RequestCommit
  ): ResponseCommit = {
    implicit val log: Log[F] = LogFactory[F].init("abci", "requestCommit").toIO.unsafeRunSync()

    ResponseCommit
      .newBuilder()
      .setData(
        ByteString.copyFrom(service.commit.toIO.unsafeRunSync().toArray)
      )
      .build()
  }

  override def requestQuery(
    req: RequestQuery
  ): ResponseQuery = {
    implicit val log: Log[F] = LogFactory[F].init("abci", "requestQuery").toIO.unsafeRunSync()

    service
      .query(req.getPath)
      .toIO
      .flatMap {
        case QueryResponse(height, result, code, info) ⇒
          log
            .info(s"${req.getPath} -> height: $height code: $code ${info.replace('\n', ' ')} ${result.length}")
            .toIO
            .map(
              _ =>
                ResponseQuery
                  .newBuilder()
                  .setCode(code.id)
                  .setInfo(info)
                  .setHeight(height)
                  .setValue(ByteString.copyFrom(result))
                  .build
            )
      }
      .unsafeRunSync()
  }

  // At the end of block H, we can propose validator updates, they will be applied at block H+2
  // see https://github.com/tendermint/tendermint/blob/master/docs/spec/abci/abci.md#endblock
  // Since Tendermint 0.30.0 all updates are processed as a set, not one by one. See https://github.com/tendermint/tendermint/issues/3222
  override def requestEndBlock(
    req: RequestEndBlock
  ): ResponseEndBlock = {
    def dropValidator(drop: DropPeer) = {
      ValidatorUpdate
        .newBuilder()
        .setPubKey(
          PubKey
            .newBuilder()
            .setType(DropPeer.KeyType)
            .setData(ByteString.copyFrom(drop.validatorKey.toArray))
        )
        .setPower(0) // settings power to zero votes to remove the validator
    }
    controlSignals.dropPeers
      .use(
        drops =>
          Applicative[F].pure {
            drops
              .foldLeft(ResponseEndBlock.newBuilder()) {
                case (resp, drop) ⇒ resp.addValidatorUpdates(dropValidator(drop))
              }
              .build()
          }
      )
      .toIO
      .unsafeRunSync()
  }
}

object AbciHandler {
  case class Config(moduleFilenames: NonEmptyList[String], abciPort: Int, blockUploadingEnabled: Boolean)

  def make[F[_]: Log: LogFactory: Effect](
    config: Config,
    statusDef: Deferred[F, F[StateMachineStatus]],
    signals: ControlSignals[F]
  ): Resource[F, Unit] =
    Resource
      .make(
        buildAbciHandler(config, statusDef, signals).value.flatMap {
          case Right(handler) ⇒ handler.pure[F]
          case Left(err) ⇒
            val exception = err.causedBy match {
              case Some(caused) => new RuntimeException("Building ABCI handler failed: " + err, caused)
              case None         => new RuntimeException("Building ABCI handler failed: " + err)
            }
            Sync[F].raiseError[AbciHandler[F]](exception)
        }.flatMap { handler ⇒
          Log[F].info("Starting State Machine ABCI handler") >>
            Sync[F].delay {
              val socket = new TSocket
              socket.registerListener(handler)

              val socketThread = new Thread(() => socket.start(config.abciPort))
              socketThread.setName("AbciSocket")
              socketThread.start()

              (socketThread, socket)
            }
        }
      ) {
        case (socketThread, socket) ⇒
          Log[F].info(s"Stopping TSocket and its thread") *>
            Sync[F].delay {
              socket.stop()
              if (socketThread.isAlive) socketThread.interrupt()
            }
      }
      .flatMap(_ ⇒ Log.resource[F].info("State Machine ABCI handler started successfully"))

  /**
   * Builds [[AbciHandler]], used to serve all Tendermint requests.
   *
   * @param config config object to load various settings
   */
  private[statemachine] def buildAbciHandler[F[_]: Log: LogFactory: Effect](
    config: Config,
    statusDef: Deferred[F, F[StateMachineStatus]],
    signals: ControlSignals[F]
  ): EitherT[F, StateMachineError, AbciHandler[F]] =
    for {
      _ ← Log.eitherT[F, StateMachineError].info("Loading VM modules from " + config.moduleFilenames)
      vm <- StateMachineVm[F](config.moduleFilenames)

      defaultStatus = StateMachineStatus(expectsEth = vm.expectsEth, stateHash = StateHash.empty)

      _ ← EitherT.right(statusDef.complete(signals.lastStateHash.map(sh ⇒ defaultStatus.copy(stateHash = sh))))

      _ ← Log.eitherT[F, StateMachineError].info("VM instantiated")

      vmInvoker = new WasmVmOperationInvoker[F](vm)

      service <- EitherT.right(StateService[F](vmInvoker, signals, config.blockUploadingEnabled))
    } yield new AbciHandler[F](service, signals)
}
