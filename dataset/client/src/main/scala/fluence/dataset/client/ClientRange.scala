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

package fluence.dataset.client

import cats.effect.Effect
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import com.google.protobuf.ByteString
import fluence.btree.core.{Hash, Key}
import fluence.btree.protocol.BTreeRpc
import fluence.dataset.protocol.ServerError
import fluence.dataset.protobuf._
import monix.execution.Scheduler
import monix.reactive.{Observable, Pipe}

import scala.collection.Searching
import scala.language.higherKinds

class ClientRange[F[_]: Effect](datasetId: Array[Byte], version: Long, rangeCallbacks: BTreeRpc.SearchCallback[F])
    extends slogging.LazyLogging {

  /** Puts error to client error(for returning error to user of this client), and return reply with error for server.*/
  private def handleClientErr(err: Throwable): F[RangeCallbackReply] = {
    Effect[F].pure(RangeCallbackReply(RangeCallbackReply.Reply.ClientError(Error(err.getMessage))))
  }

  private def handleAsks(source: Observable[RangeCallback.Callback]): Observable[RangeCallbackReply] =
    source.collect { case ask if ask.isDefined && !ask.isValue && !ask.isServerError ⇒ ask } // Collect callbacks
      .mapEval[F, RangeCallbackReply] {

        case ask if ask.isNextChildIndex ⇒
          val Some(nci) = ask.nextChildIndex

          rangeCallbacks
            .nextChildIndex(
              nci.keys.map(k ⇒ Key(k.toByteArray)).toArray,
              nci.childsChecksums.map(c ⇒ Hash(c.toByteArray)).toArray
            )
            .attempt
            .flatMap {
              case Left(err) ⇒
                handleClientErr(err)
              case Right(idx) ⇒
                Effect[F].pure(RangeCallbackReply(RangeCallbackReply.Reply.NextChildIndex(ReplyNextChildIndex(idx))))
            }

        case ask if ask.isSubmitLeaf ⇒
          val Some(sl) = ask.submitLeaf

          rangeCallbacks
            .submitLeaf(
              sl.keys.map(k ⇒ Key(k.toByteArray)).toArray,
              sl.valuesChecksums.map(c ⇒ Hash(c.toByteArray)).toArray
            )
            .attempt
            .flatMap {
              case Left(err) ⇒
                handleClientErr(err)
              case Right(searchResult) ⇒
                Effect[F].pure(
                  RangeCallbackReply(
                    RangeCallbackReply.Reply.SubmitLeaf(
                      ReplySubmitLeaf(
                        searchResult match {
                          case Searching.Found(i) ⇒ ReplySubmitLeaf.SearchResult.Found(i)
                          case Searching.InsertionPoint(i) ⇒ ReplySubmitLeaf.SearchResult.InsertionPoint(i)
                        }
                      )
                    )
                  )
                )
            }

      }

  /**
   * Run client bidi stream.
   *
   * @param pipe Bidi pipe for transport layer
   * @return
   */
  def runStream(
    pipe: Pipe[RangeCallbackReply, RangeCallback]
  )(implicit sch: Scheduler): Observable[(Array[Byte], Array[Byte])] = {
    // Get observer/observable for request's bidiflow
    val (pushClientReply, pullServerAsk) = pipe
      .transform(_.map {
        case RangeCallback(callback) ⇒
          logger.trace(s"DatasetStorageClient.range() received server ask=$callback")
          callback
      })
      .multicast

    (
      Observable(
        RangeCallbackReply(
          RangeCallbackReply.Reply.DatasetInfo(DatasetInfo(ByteString.copyFrom(datasetId), version))
        )
      ) ++ handleAsks(pullServerAsk)
    ).subscribe(pushClientReply) // And clientReply response back to server

    pullServerAsk.collect {
      case ask if ask.isServerError ⇒
        val Some(err) = ask.serverError
        val serverError = ServerError(err.msg)
        // if server send an error we should close stream and lift error up
        Observable(pushClientReply.onError(serverError))
          .flatMap(_ ⇒ Observable.raiseError[(Array[Byte], Array[Byte])](serverError))
      case ask if ask.isValue ⇒
        val Some(RangeValue(key, value)) = ask._value
        logger.trace(s"DatasetStorageClient.range() received server value=$value for key=$key")
        Observable(key.toByteArray → value.toByteArray)

    }.flatten
  }

}

object ClientRange extends slogging.LazyLogging {

  /**
   * Prepare ''Range'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param version Dataset version expected to the client
   * @param rangeCallbacks Wrapper for all callback needed for ''Range'' operation to the BTree
   * @return returns stream of found value.
   */
  def apply[F[_]: Effect](
    datasetId: Array[Byte],
    version: Long,
    rangeCallbacks: BTreeRpc.SearchCallback[F]
  ): ClientRange[F] = {
    new ClientRange(datasetId, version, rangeCallbacks)
  }
}
