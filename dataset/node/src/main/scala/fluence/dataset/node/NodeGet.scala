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
import fluence.btree.core.{Hash, Key}
import fluence.btree.protocol.BTreeRpc
import fluence.dataset.protocol.{ClientError, DatasetStorageRpc}
import fluence.protobuf.dataset._
import monix.eval.Task
import monix.execution.{Ack, Scheduler}
import monix.reactive.{Observable, Observer}

import scala.collection.Searching
import scala.language.higherKinds

class NodeGet[F[_]: Async](service: DatasetStorageRpc[F, Observable])(
  implicit
  runF: F ~> Task,
  scheduler: Scheduler
) extends slogging.LazyLogging {
  import DatasetServerUtils._

  private def getReply[T](
    pullClientReply: () ⇒ Task[GetCallbackReply],
    check: GetCallbackReply.Reply ⇒ Boolean,
    extract: GetCallbackReply.Reply ⇒ T
  ): EitherT[Task, ClientError, T] = {

    val clReply = pullClientReply().map {
      case GetCallbackReply(reply) ⇒
        logger.trace(s"DatasetStorageServer.get() received client reply=$reply")
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
    resp: Observer[GetCallback],
    pullClientReply: () ⇒ Task[GetCallbackReply]
  ): F[Option[Array[Byte]]] =
    for {
      datasetInfo ← toF(getReply(pullClientReply, _.isDatasetInfo, _.datasetInfo.get))
      foundValue ← service.get(
        datasetInfo.id.toByteArray,
        datasetInfo.version,
        new BTreeRpc.SearchCallback[F] {

          private val pushServerAsk: GetCallback.Callback ⇒ EitherT[Task, ClientError, Ack] = callback ⇒ {
            EitherT(Task.fromFuture(resp.onNext(GetCallback(callback = callback))).attempt)
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
                  GetCallback.Callback.SubmitLeaf(
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
                  GetCallback.Callback.NextChildIndex(
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
      logger.debug(s"Was found value=${foundValue.show} for client 'get' request for dataset=${datasetInfo.show}")
      foundValue
    }

  def runStream(resp: Observer[GetCallback], repl: Observable[GetCallbackReply]): Unit = {
    val pullClientReply: () ⇒ Task[GetCallbackReply] = pullable(repl)

    // Launch service call, push the value once it's received
    resp completeWith runF(
      valueF(resp, pullClientReply).attempt.flatMap {
        case Right(value) ⇒
          // if all is ok server should close the stream (is done in ObserverGrpcOps.completeWith) and send value to client
          Async[F].pure(
            GetCallback(GetCallback.Callback.Value(GetValue(value.fold(ByteString.EMPTY)(ByteString.copyFrom))))
          )
        case Left(clientError: ClientError) ⇒
          logger.warn(s"Client replied with an error=$clientError")
          // when server receive client error, server shouldn't close the stream (is done in ObserverGrpcOps.completeWith) and lift up client error
          Async[F].raiseError[GetCallback](clientError)
        case Left(exception) ⇒
          // when server error appears, server should log it and send to client
          logger.warn(s"Server threw an exception=$exception and sends cause to client")
          GetCallback(GetCallback.Callback.ServerError(Error(exception.getMessage))).pure[F]
      }
    )
  }
}

object NodeGet extends slogging.LazyLogging {

  def apply[F[_]: Async](
    service: DatasetStorageRpc[F, Observable]
  )(
    implicit
    runF: F ~> Task,
    scheduler: Scheduler
  ): NodeGet[F] = {
    new NodeGet(service)
  }
}
