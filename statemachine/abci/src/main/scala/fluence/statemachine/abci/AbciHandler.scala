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
import fluence.bp.tx._
import fluence.log.{Log, LogFactory}
import fluence.statemachine.abci.peers.{DropPeer, PeersControlBackend}
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.command.TxProcessor
import fluence.statemachine.api.query.QueryResponse
import io.circe.syntax._
import shapeless._

import scala.language.higherKinds
import scala.util.Try

class AbciHandler[F[_]: Effect: LogFactory] private (
  machine: StateMachine[F],
  peersControl: PeersControlBackend[F],
  txHandler: TxProcessor[F]
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

    txHandler
      .checkTx(tx)
      .value
      .flatMap {
        case Right(resp) ⇒
          handleTxResponse(tx, resp)
            .map(
              _ =>
                ResponseCheckTx.newBuilder
                  .setCode(resp.code.id)
                  .setData(ByteString.copyFromUtf8(resp.asJson.noSpaces))
                  .build
            )
        case Left(err) ⇒
          Sync[F].raiseError[ResponseCheckTx](err)
      }
      .toIO
      .unsafeRunSync()
  }

  override def receivedDeliverTx(
    req: RequestDeliverTx
  ): ResponseDeliverTx = {
    implicit val log: Log[F] = LogFactory[F].init("abci", "requestDeliverTx").toIO.unsafeRunSync()

    val tx = req.getTx.toByteArray

    txHandler
      .processTx(tx)
      .value
      .flatMap {
        case Right(resp) ⇒
          handleTxResponse(tx, resp)
            .map(
              _ =>
                ResponseDeliverTx.newBuilder
                  .setCode(resp.code.id)
                  .setData(ByteString.copyFromUtf8(resp.asJson.noSpaces))
                  .build
            )
        case Left(err) ⇒
          Sync[F].raiseError[ResponseDeliverTx](err)
      }
      .toIO
      .unsafeRunSync()
  }

  override def requestCommit(
    requestCommit: RequestCommit
  ): ResponseCommit = {
    implicit val log: Log[F] = LogFactory[F].init("abci", "requestCommit").toIO.unsafeRunSync()

    txHandler.commit.value.flatMap {
      case Right(stateHash) ⇒
        stateHash.hash.toArray.pure[F]
      case Left(err) ⇒
        Sync[F].raiseError[Array[Byte]](err)
    }.map(ByteString.copyFrom)
      .map(
        ResponseCommit
          .newBuilder()
          .setData(
            _
          )
          .build()
      )
      .toIO
      .unsafeRunSync()
  }

  override def requestQuery(
    req: RequestQuery
  ): ResponseQuery = {
    implicit val log: Log[F] = LogFactory[F].init("abci", "requestQuery").toIO.unsafeRunSync()

    machine
      .query(req.getPath)
      .value
      .flatMap {
        case Right(QueryResponse(height, result, code, info)) ⇒
          log
            .info(s"${req.getPath} -> height: $height code: $code ${info.replace('\n', ' ')} ${result.length}")
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
        case Left(err) ⇒
          Sync[F].raiseError[ResponseQuery](err)
      }
      .toIO
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
    peersControl.dropPeers
      .use(
        drops =>
          drops
            .foldLeft(ResponseEndBlock.newBuilder()) {
              case (resp, drop) ⇒ resp.addValidatorUpdates(dropValidator(drop))
            }
            .build()
            .pure[F]
      )
      .toIO
      .unsafeRunSync()
  }
}

object AbciHandler {

  def make[F[_]: Log: LogFactory: Effect, C <: HList](
    abciPort: Int,
    machine: StateMachine.Aux[F, C],
    peersControl: PeersControlBackend[F]
  )(
    implicit txp: ops.hlist.Selector[C, TxProcessor[F]]
  ): Resource[F, Unit] =
    Resource
      .make(
        Log[F].info("Starting State Machine ABCI handler") >>
          Sync[F].delay {
            val handler = apply[F, C](machine, peersControl)

            val socket = new TSocket
            socket.registerListener(handler)

            val socketThread = new Thread(() => socket.start(abciPort))
            socketThread.setName("AbciSocket")
            socketThread.start()

            (socketThread, socket)
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
   * Builds [[AbciHandler]], used to serve all Tendermint requests
   */
  def apply[F[_]: Log: LogFactory: Effect, C <: HList](
    machine: StateMachine.Aux[F, C],
    peersControl: PeersControlBackend[F]
  )(
    implicit txp: ops.hlist.Selector[C, TxProcessor[F]]
  ): AbciHandler[F] =
    new AbciHandler[F](machine, peersControl, machine.command[TxProcessor[F]])
}
