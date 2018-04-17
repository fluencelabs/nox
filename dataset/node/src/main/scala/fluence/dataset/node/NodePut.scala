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

package fluence.dataset.node

import cats.data.EitherT
import cats.effect.Async
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.applicative._
import cats.syntax.show._
import cats.~>
import com.google.protobuf.ByteString
import fluence.btree.core.{ClientPutDetails, Hash, Key}
import fluence.btree.protocol.BTreeRpc
import fluence.dataset.protocol.{ClientError, DatasetStorageRpc}
import fluence.protobuf.dataset._
import monix.eval.Task
import monix.execution.{Ack, Scheduler}
import monix.reactive.{Observable, Observer}
import scodec.bits.ByteVector

import scala.collection.Searching
import scala.language.higherKinds

class NodePut[F[_]: Async](service: DatasetStorageRpc[F, Observable])(
  implicit
  runF: F ~> Task,
  scheduler: Scheduler
) extends slogging.LazyLogging {

  import DatasetServerUtils._

  private def getReply[T](
    pullClientReply: () ⇒ Task[PutCallbackReply],
    check: PutCallbackReply.Reply ⇒ Boolean,
    extract: PutCallbackReply.Reply ⇒ T
  ): EitherT[Task, ClientError, T] = {

    val reply = pullClientReply().map {
      case PutCallbackReply(r) ⇒
        logger.trace(s"DatasetStorageServer.put() received client reply=$r")
        r
    }.map {
      case r if check(r) ⇒
        Right(extract(r))
      case r ⇒
        val errMsg = r.clientError.map(_.msg).getOrElse("Wrong reply received, protocol error")
        Left(ClientError(errMsg))
    }

    EitherT(reply)
  }

  private def valueF(pullClientReply: () ⇒ Task[PutCallbackReply], resp: Observer[PutCallback]) =
    for {
      datasetInfo ← toF(getReply(pullClientReply, _.isDatasetInfo, _.datasetInfo.get))
      putValue ← toF(
        getReply(pullClientReply, _.isValue, _._value.map(_.value.toByteArray).getOrElse(Array.emptyByteArray))
      )
      oldValue ← service.put(
        datasetInfo.id.toByteArray,
        datasetInfo.version,
        new BTreeRpc.PutCallbacks[F] {

          private val pushServerAsk: PutCallback.Callback ⇒ EitherT[Task, ClientError, Ack] = callback ⇒ {
            EitherT(Task.fromFuture(resp.onNext(PutCallback(callback = callback))).attempt)
              .leftMap(t ⇒ ClientError(t.getMessage))
          }

          /**
           * Server asks next child node index.
           *
           * @param keys            Keys of current branch for searching index
           * @param childsChecksums All children checksums of current branch
           */
          override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Hash]): F[Int] =
            toF(
              for {
                _ ← pushServerAsk(
                  PutCallback.Callback.NextChildIndex(
                    AskNextChildIndex(
                      keys = keys.map(k ⇒ ByteString.copyFrom(k.bytes)),
                      childsChecksums = childsChecksums.map(c ⇒ ByteString.copyFrom(c.bytes))
                    )
                  )
                )
                nci ← getReply(pullClientReply, _.isNextChildIndex, _.nextChildIndex.get)
              } yield nci.index
            )

          /**
           * Server sends founded leaf details.
           *
           * @param keys            Keys of current leaf
           * @param valuesChecksums Checksums of values for current leaf
           */
          override def putDetails(keys: Array[Key], valuesChecksums: Array[Hash]): F[ClientPutDetails] =
            toF(
              for {
                _ ← pushServerAsk(
                  PutCallback.Callback.PutDetails(
                    AskPutDetails(
                      keys = keys.map(k ⇒ ByteString.copyFrom(k.bytes)),
                      valuesChecksums = valuesChecksums.map(c ⇒ ByteString.copyFrom(c.bytes))
                    )
                  )
                )
                pd ← getReply(
                  pullClientReply,
                  r ⇒ r.isPutDetails && r.putDetails.exists(_.searchResult.isDefined),
                  _.putDetails.get
                )
              } yield
                ClientPutDetails(
                  key = Key(pd.key.toByteArray),
                  valChecksum = Hash(pd.checksum.toByteArray),
                  searchResult = (
                    pd.searchResult.found.map(Searching.Found) orElse
                      pd.searchResult.insertionPoint.map(Searching.InsertionPoint)
                  ).get
                )
            )

          /**
           * Server sends new merkle root to client for approve made changes.
           *
           * @param serverMerkleRoot New merkle root after putting key/value
           * @param wasSplitting     'True' id server performed tree rebalancing, 'False' otherwise
           */
          override def verifyChanges(serverMerkleRoot: Hash, wasSplitting: Boolean): F[ByteVector] =
            toF(
              for {
                _ ← pushServerAsk(
                  PutCallback.Callback.VerifyChanges(
                    AskVerifyChanges(
                      serverMerkleRoot = ByteString.copyFrom(serverMerkleRoot.bytes),
                      splitted = wasSplitting
                    )
                  )
                )
                clientSignature ← getReply(pullClientReply, _.isVerifyChanges, _.verifyChanges.get.signature)
              } yield ByteVector(clientSignature.toByteArray)
            )

          /**
           * Server confirms that all changes was persisted.
           */
          override def changesStored(): F[Unit] =
            toF(
              for {
                _ ← pushServerAsk(PutCallback.Callback.ChangesStored(AskChangesStored()))
                _ ← getReply(pullClientReply, _.isChangesStored, _.changesStored.get)
              } yield ()
            )
        },
        putValue
      )
    } yield {
      logger.debug(
        s"Was stored new value=${putValue.show} for client 'put' request for ${datasetInfo.show}" +
          s" old value=${oldValue.show} was overwritten"
      )
      oldValue
    }

  def runStream(resp: Observer[PutCallback], repl: Observable[PutCallbackReply]): Unit = {
    val pullClientReply: () ⇒ Task[PutCallbackReply] = pullable(repl)

    // Launch service call, push the value once it's received
    resp completeWith runF(
      valueF(pullClientReply, resp).attempt.flatMap {
        case Right(value) ⇒
          // if all is ok server should close the stream(is done in ObserverGrpcOps.completeWith)  and send value to client
          Async[F].pure(
            PutCallback(PutCallback.Callback.Value(PreviousValue(value.fold(ByteString.EMPTY)(ByteString.copyFrom))))
          )
        case Left(clientError: ClientError) ⇒
          // when server receive client error, server shouldn't close the stream(is done in ObserverGrpcOps.completeWith)  and lift up client error
          Async[F].raiseError[PutCallback](clientError)
        case Left(exception) ⇒
          // when server error appears, server should log it and send to client
          logger.warn(s"Severs throw an exception=($exception) and send cause to client")
          PutCallback(PutCallback.Callback.ServerError(Error(exception.getMessage))).pure[F]
      }
    )
  }

}

object NodePut extends slogging.LazyLogging {

  def apply[F[_]: Async](
    service: DatasetStorageRpc[F, Observable],
    resp: Observer[PutCallback],
    repl: Observable[PutCallbackReply]
  )(
    implicit
    runF: F ~> Task,
    scheduler: Scheduler
  ): NodePut[F] = {
    new NodePut(service)
  }
}
