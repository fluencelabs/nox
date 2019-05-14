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
import cats.data.EitherT
import cats.effect.concurrent.Ref
import cats.effect.syntax.effect._
import cats.effect.{Effect, IO}
import cats.syntax.eq._
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.github.jtendermint.jabci.api._
import com.github.jtendermint.jabci.types._
import com.google.protobuf.ByteString
import fluence.effects.tendermint.block.TendermintBlock
import fluence.effects.tendermint.block.errors.Errors._
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.statemachine.control.{ControlSignals, DropPeer}
import io.circe.{Json, ParsingFailure}

import scala.language.higherKinds
import scala.util.Try

class AbciHandler[F[_]: Effect](
  service: AbciService[F],
  controlSignals: ControlSignals[F],
  tendermintRpc: TendermintRpc[F],
  blocks: Ref[F, Map[Long, (String, Json)]],
  commits: Ref[F, Map[Long, (String, Json)]]
) extends ICheckTx with IDeliverTx with ICommit with IQuery with IEndBlock with IBeginBlock
    with slogging.LazyLogging {

  private def parseJson(json: String): Either[ParsingFailure, Json] = {
    import io.circe.parser._
    parse(json)
  }

  private def getJsons(height: Long): F[(Json, Json)] = {
    val jsonsF = for {
      block <- tendermintRpc.block(height)               .leftMap(new Exception(_): Throwable)
      blockJson <- EitherT.fromEither(parseJson(block))  .leftMap(new Exception(_): Throwable)
      commit <- tendermintRpc.commit(height)             .leftMap(new Exception(_): Throwable)
      commitJson <- EitherT.fromEither(parseJson(commit)).leftMap(new Exception(_): Throwable)
    } yield (blockJson, commitJson)

    jsonsF.value.flatMap(Effect[F].fromEither)
  }

  private def store(height: Long, method: String) = {
    logger.info(s"Store[$height] began")
    for {
      tuple <- getJsons(height)
      (blockJson, commitJson) = tuple
      _ <- blocks.update(_.updated(height, method -> blockJson))
      _ <- commits.update(_.updated(height, method -> commitJson))
      _ = logger.info(s"Store[$height] completed")
    } yield ()
  }

  private def getDiff(left: Json, right: Json) = {
    import diffson._
    import diffson.circe._
    import diffson.jsonpatch.lcsdiff._
    import diffson.lcs._
    import io.circe._

    implicit val lcs = new Patience[Json]

    diff(left, right)
  }

  private def compare(height: Long, method: String) = {
    logger.info(s"Compare[$height] began")
    for {
      tuple <- getJsons(height)
      (blockJson, commitJson) = tuple
      storedBlock <- blocks.get.map(_.get(height))
      storedCommit <- commits.get.map(_.get(height))
    } yield {
      storedBlock match {
        case None => logger.info(s"Block[$height] $method: stored block is empty")
        case Some((storeMethod, storedBlock)) if storedBlock =!= blockJson =>
          val diff = Try(getDiff(storedBlock, blockJson))
          logger.info(s"Block[$height] $storeMethod EQ $method => FALSE\nDIFF:\n$diff")
        case Some((storeMethod, storedBlock)) =>
          logger.info(s"Block[$height] $storeMethod EQ $method => TRUE")
      }

      storedCommit match {
        case None => logger.info(s"Commit[$height] $method: stored commit is empty")
        case Some((storeMethod, storedCommit)) if storedCommit =!= commitJson =>
          val diff = Try(getDiff(storedCommit, commitJson))
          logger.info(s"Commit[$height] $storeMethod EQ $method => FALSE\nDIFF:\n$diff")
        case Some((storeMethod, storedCommit)) =>
          logger.info(s"Commit[$height] $storeMethod EQ $method => TRUE")
      }
    }
  }

  private def checkBlock(height: Long): Unit = {
    val log: String ⇒ Unit = s ⇒ logger.info(Console.YELLOW + s + Console.RESET)
    val logBad: String ⇒ Unit = s ⇒ logger.info(Console.RED + s + Console.RESET)

    tendermintRpc
      .block(height)
      .value
      .toIO
      .map(
        res =>
          for {
            str <- res.leftTap(e => logger.warn(s"RPC Block[$height] failed: $e"))
            block <- TendermintBlock(str).leftTap(e => logBad(s"Failed to decode tendermint block from JSON: $e"))
            _ = logger.info(s"RPC Block[$height] => height = ${block.block.header.height}")
            _ <- block.validateHashes().leftTap(e => logBad(s"Block at height $height is invalid: $e"))
          } yield log(s"Block at height $height is valid")
      )
      .unsafeRunAsyncAndForget()
  }

  override def requestBeginBlock(
    req: RequestBeginBlock
  ): ResponseBeginBlock = {
    val height = req.getHeader.getHeight
    checkBlock(height)

    store(height, "BeginBlock").toIO.handleErrorWith { e =>
      IO.pure(logger.error("Error while storing block in BeginBlock: " + e))
    }.unsafeRunSync()

    ResponseBeginBlock.newBuilder().build()
  }

  override def requestCheckTx(
    req: RequestCheckTx
  ): ResponseCheckTx =
    service
      .checkTx(req.getTx.toByteArray)
      .toIO
      .map {
        case AbciService.TxResponse(code, info) ⇒
          ResponseCheckTx.newBuilder
            .setCode(code)
            .setInfo(info)
            // TODO where it goes?
            .setData(ByteString.copyFromUtf8(info))
            .build
      }
      .unsafeRunSync()

  override def receivedDeliverTx(
    req: RequestDeliverTx
  ): ResponseDeliverTx =
    service
      .deliverTx(req.getTx.toByteArray)
      .toIO
      .map {
        case AbciService.TxResponse(code, info) ⇒
          ResponseDeliverTx.newBuilder
            .setCode(code)
            .setInfo(info)
            // TODO where it goes?
            .setData(ByteString.copyFromUtf8(info))
            .build
      }
      .unsafeRunSync()

  override def requestCommit(
    requestCommit: RequestCommit
  ): ResponseCommit = {
    service.stateHeight
      .flatMap(h => compare(h, "RequestCommit"))
      .toIO
      .handleErrorWith { e =>
        IO.pure(logger.error("Error comparing in RequestCommit: " + e))
      }
      .unsafeRunSync()
    logger.info("After compare in RequestCommit")

    ResponseCommit
      .newBuilder()
      .setData(
        ByteString.copyFrom(service.commit.toIO.unsafeRunSync().toArray)
      )
      .build()
  }

  override def requestQuery(
    req: RequestQuery
  ): ResponseQuery =
    service
      .query(req.getPath)
      .toIO
      .map {
        case AbciService.QueryResponse(height, result, code, info) ⇒
          ResponseQuery
            .newBuilder()
            .setCode(code)
            .setInfo(info)
            .setHeight(height)
            .setValue(ByteString.copyFrom(result))
            .build
      }
      .unsafeRunSync()

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
