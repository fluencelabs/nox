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

import cats.Applicative
import cats.effect.Effect
import cats.effect.syntax.effect._
import cats.syntax.flatMap._
import com.github.jtendermint.jabci.api._
import com.github.jtendermint.jabci.types._
import com.google.protobuf.ByteString
import fluence.log.{Log, LogFactory}
import fluence.statemachine.control.{ControlSignals, DropPeer}
import fluence.statemachine.data.Tx

import scala.language.higherKinds
import scala.util.Try

class AbciHandler[F[_]: Effect: LogFactory](
  service: AbciService[F],
  controlSignals: ControlSignals[F],
) extends ICheckTx with IDeliverTx with ICommit with IQuery with IEndBlock with IBeginBlock {

  override def requestBeginBlock(
    req: RequestBeginBlock
  ): ResponseBeginBlock = ResponseBeginBlock.newBuilder().build()

  override def requestCheckTx(
    req: RequestCheckTx
  ): ResponseCheckTx = {
    implicit val log: Log[F] = LogFactory[F].init("abci", "requestCheckTx").toIO.unsafeRunSync()

    val tx = req.getTx.toByteArray

    service
      .checkTx(tx)
      .toIO
      .flatMap {
        case AbciService.TxResponse(code, info, height) ⇒
          log
            .info(
              Tx.splitTx(tx)
                .fold(
                  s"${height.fold("")(h => s"height $height")} can't parse head from tx ${Try(new String(tx.take(100)))}"
                ) {
                  case (head, _) =>
                    s"tx.head: $head -> $code ${info.replace('\n', ' ')} ${height.fold("")(h => s"height $height")}"
                }
            )
            .toIO
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
        case AbciService.TxResponse(code, info, height) ⇒
          log
            .info(
              Tx.splitTx(tx)
                .fold(
                  s"${height.fold("")(h => s"height $height")} can't parse head from tx ${Try(new String(tx.take(100)))}"
                ) {
                  case (head, _) =>
                    s"tx.head: $head -> $code ${info.replace('\n', ' ')} ${height.fold("")(h => s"height $height")}"
                }
            )
            .toIO
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
        case AbciService.QueryResponse(height, result, code, info) ⇒
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
            .setType(DropPeer.KEY_TYPE)
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
