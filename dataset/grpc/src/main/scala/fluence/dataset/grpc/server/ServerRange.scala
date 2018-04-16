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

package fluence.dataset.grpc.server

import cats.data.EitherT
import cats.effect.Async
import cats.syntax.applicativeError._
import cats.syntax.show._
import cats.~>
import com.google.protobuf.ByteString
import fluence.btree.core.{Hash, Key}
import fluence.btree.protocol.BTreeRpc
import fluence.dataset.grpc.client.ClientError
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.protobuf.dataset._
import monix.eval.Task
import monix.execution.{Ack, Cancelable, Scheduler}
import monix.reactive.{Observable, Observer}

import scala.collection.Searching

class ServerRange[F[_]: Async](service: DatasetStorageRpc[F, Observable])(
  implicit
  runF: F ~> Task,
  scheduler: Scheduler
) extends slogging.LazyLogging {

  import DatasetServerUtils._

  private def getReply[T](
    pullClientReply: () ⇒ Task[RangeCallbackReply],
    check: RangeCallbackReply.Reply ⇒ Boolean,
    extract: RangeCallbackReply.Reply ⇒ T
  ): EitherT[Task, ClientError, T] = {

    val clReply = pullClientReply().map {
      case RangeCallbackReply(reply) ⇒
        logger.trace(s"DatasetStorageServer.range() received client reply=$reply")
        reply
    }.map {
      case r if check(r) ⇒
        Right(extract(r))
      case r ⇒
        val errMsg = r.clientError.map(_.msg).getOrElse("Wrong reply received, protocol error")
        Left(ClientError(errMsg))
    }

    EitherT(clReply)
  }

  private def valueF(
    pullClientReply: () ⇒ Task[RangeCallbackReply],
    resp: Observer[RangeCallback]
  ): Observable[(Array[Byte], Array[Byte])] =
    for {
      datasetInfo ← toObservable(getReply(pullClientReply, _.isDatasetInfo, _.datasetInfo.get))
      valuesStream ← service.range(
        datasetInfo.id.toByteArray,
        datasetInfo.version,
        new BTreeRpc.SearchCallback[F] {

          private val pushServerAsk: RangeCallback.Callback ⇒ EitherT[Task, ClientError, Ack] = callback ⇒ {
            EitherT(Task.fromFuture(resp.onNext(RangeCallback(callback = callback))).attempt)
              .leftMap(t ⇒ ClientError(t.getMessage))
          }

          /**
           * Server sends founded leaf details.
           *
           * @param keys            Keys of current leaf
           * @param valuesChecksums Checksums of values for current leaf
           * @return index of searched value, or None if key wasn't found
           */
          override def submitLeaf(keys: Array[Key], valuesChecksums: Array[Hash]): F[Searching.SearchResult] =
            toF(
              for {
                _ ← pushServerAsk(
                  RangeCallback.Callback.SubmitLeaf(
                    AskSubmitLeaf(
                      keys = keys.map(k ⇒ ByteString.copyFrom(k.bytes)),
                      valuesChecksums = valuesChecksums.map(c ⇒ ByteString.copyFrom(c.bytes))
                    )
                  )
                )
                sl ← getReply(pullClientReply, _.isSubmitLeaf, _.submitLeaf.get)
              } yield {
                sl.searchResult.found
                  .map(Searching.Found)
                  .orElse(sl.searchResult.insertionPoint.map(Searching.InsertionPoint))
                  .get
              }
            )

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
                  RangeCallback.Callback.NextChildIndex(
                    AskNextChildIndex(
                      keys = keys.map(k ⇒ ByteString.copyFrom(k.bytes)),
                      childsChecksums = childsChecksums.map(c ⇒ ByteString.copyFrom(c.bytes))
                    )
                  )
                )
                nci ← getReply(pullClientReply, _.isNextChildIndex, _.nextChildIndex.get)
              } yield nci.index
            )
        }
      )
    } yield {
      logger.debug(s"Was found value=${valuesStream.show} for client 'range' request for ${datasetInfo.show}")
      valuesStream
    }

  def runStream(resp: Observer[RangeCallback], repl: Observable[RangeCallbackReply]): Cancelable = {
    val pullClientReply: () ⇒ Task[RangeCallbackReply] = pullable(repl)

    valueF(pullClientReply, resp).attempt.flatMap {
      case Right((key, value)) ⇒
        // if all is ok server should send value to client
        Observable.pure(
          RangeCallback(RangeCallback.Callback.Value(RangeValue(ByteString.copyFrom(key), ByteString.copyFrom(value))))
        )
      case Left(clientError: ClientError) ⇒
        logger.warn(s"Client replied with an error=$clientError")
        // if server receive client error server should lift it up
        Observable.raiseError(clientError)
      case Left(exception) ⇒
        // when server error appears, server should log it and send to client
        logger.warn(s"Server threw an exception=$exception and sends cause to client")
        Observable.pure(RangeCallback(RangeCallback.Callback.ServerError(Error(exception.getMessage))))
    }.subscribe(resp)
  }
}

object ServerRange extends slogging.LazyLogging {

  def apply[F[_]: Async](
    service: DatasetStorageRpc[F, Observable],
    resp: Observer[RangeCallback],
    repl: Observable[RangeCallbackReply]
  )(
    implicit
    runF: F ~> Task,
    scheduler: Scheduler
  ): ServerRange[F] = {
    new ServerRange(service)
  }
}
