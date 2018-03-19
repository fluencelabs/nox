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

package fluence.dataset.grpc

import cats.effect.Effect
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import com.google.protobuf.ByteString
import fluence.btree.core.{ Hash, Key }
import fluence.btree.protocol.BTreeRpc
import fluence.dataset.grpc.DatasetStorageClient.ServerError
import fluence.dataset.grpc.DatasetStorageServer.ClientError
import fluence.dataset.grpc.GrpcMonix._
import fluence.dataset.grpc.DatasetStorageRpcGrpc.DatasetStorageRpcStub
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.transport.grpc.client.GrpcClient
import io.grpc.{ CallOptions, ManagedChannel }
import monix.eval.{ MVar, Task }
import monix.execution.Scheduler
import monix.reactive.Observable

import scala.collection.Searching
import scala.language.{ higherKinds, implicitConversions }
import scala.util.control.NoStackTrace

/**
 * Clients implementation of [[DatasetStorageRpc]], allows talking to server via network.
 * All public methods called from the client side.
 * DatasetStorageClient initiates first request to server and then answered to server requests.
 *
 * @param stub Stub for calling server methods of [[DatasetStorageRpc]]
 * @tparam F A box for returning value
 */
class DatasetStorageClient[F[_] : Effect](
    stub: DatasetStorageRpcStub
)(implicit sch: Scheduler) extends DatasetStorageRpc[F, Observable] with slogging.LazyLogging {

  private def run[A](fa: Task[A]): F[A] = fa.toIO.to[F]

  /**
   * Initiates ''Get'' operation in remote MerkleBTree.
   *
   * @param datasetId    Dataset ID
   * @param getCallbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   * @return returns found value, None if nothing was found.
   */
  override def get(datasetId: Array[Byte], getCallbacks: BTreeRpc.SearchCallback[F]): F[Option[Array[Byte]]] = {
    // Convert a remote stub call to monix pipe
    val pipe = callToPipe(stub.get)

    // Get observer/observable for request's bidiflow
    val (pushClientReply, pullServerAsk) = pipe
      .transform(_.map {
        case GetCallback(callback) ⇒
          logger.trace(s"DatasetStorageClient.get() received server ask=$callback")
          callback
      })
      .multicast

    val clientError = MVar.empty[Throwable]

    /** Puts error to client error(for returning error to user of this client), and return reply with error for server.*/
    def handleClientErr(err: Throwable): F[GetCallbackReply] = {
      run(
        clientError
          .put(ClientError(err.getMessage))
          .map(_ ⇒ GetCallbackReply(GetCallbackReply.Reply.ClientError(Error(err.getMessage))))
      )
    }

    val handleAsks = pullServerAsk
      .collect { case ask if ask.isDefined && !ask.isValue && !ask.isServerError ⇒ ask } // Collect callbacks
      .mapEval[F, GetCallbackReply] {

        case ask if ask.isNextChildIndex ⇒
          val Some(nci) = ask.nextChildIndex

          getCallbacks
            .nextChildIndex(nci.keys.map(k ⇒ Key(k.toByteArray)).toArray, nci.childsChecksums.map(c ⇒ Hash(c.toByteArray)).toArray)
            .attempt
            .flatMap {
              case Left(err) ⇒
                handleClientErr(err)
              case Right(idx) ⇒
                Effect[F].pure(GetCallbackReply(GetCallbackReply.Reply.NextChildIndex(ReplyNextChildIndex(idx))))
            }

        case ask if ask.isSubmitLeaf ⇒
          val Some(sl) = ask.submitLeaf

          getCallbacks
            .submitLeaf(sl.keys.map(k ⇒ Key(k.toByteArray)).toArray, sl.valuesChecksums.map(c ⇒ Hash(c.toByteArray)).toArray)
            .attempt
            .flatMap {
              case Left(err) ⇒
                handleClientErr(err)
              case Right(searchResult) ⇒
                Effect[F].pure(GetCallbackReply(GetCallbackReply.Reply.SubmitLeaf(ReplySubmitLeaf(
                  searchResult match {
                    case Searching.Found(i)          ⇒ ReplySubmitLeaf.SearchResult.Found(i)
                    case Searching.InsertionPoint(i) ⇒ ReplySubmitLeaf.SearchResult.InsertionPoint(i)
                  }
                ))))
            }

      }

    (
      Observable(
        GetCallbackReply(
          GetCallbackReply.Reply.DatasetId(DatasetId(ByteString.copyFrom(datasetId)))
        )
      ) ++ handleAsks
    ).subscribe(pushClientReply) // And clientReply response back to server

    val errorOrValue =
      pullServerAsk
        .collect { // Collect terminal task with value/error
          case ask if ask.isServerError ⇒
            val Some(err) = ask.serverError
            val serverError = ServerError(err.msg)
            // if server send an error we should close stream and lift error up
            Task(pushClientReply.onError(serverError))
              .flatMap(_ ⇒ Task.raiseError[Option[Array[Byte]]](serverError))
          case ask if ask.isValue ⇒
            val Some(getValue) = ask._value
            // if got success response or server error close stream and return value\error to user of this client
            Task(pushClientReply.onComplete())
              .map { _ ⇒
                Option(getValue.value)
                  .filterNot(_.isEmpty)
                  .map(_.toByteArray)
              }
        }.headOptionL // Take the first option value or server error

    run(errorOrValue.flatMap {
      case Some(value) ⇒ value // return success result or server error in first
      case None        ⇒ clientError.take.flatMap(err ⇒ Task.raiseError(err)) // return occurred clients error
    })

  }

  /**
   * Initiates ''Range'' operation in remote MerkleBTree.
   *
   * @param datasetId       Dataset ID
   * @param rangeCallbacks Wrapper for all callback needed for ''Range'' operation to the BTree
   * @return returns stream of found value.
   */
  override def range(
    datasetId: Array[Byte],
    rangeCallbacks: BTreeRpc.SearchCallback[F]
  ): Observable[(Array[Byte], Array[Byte])] = {

    // Convert a remote stub call to monix pipe
    val pipe = callToPipe(stub.range)

    // Get observer/observable for request's bidiflow
    val (pushClientReply, pullServerAsk) = pipe
      .transform(_.map {
        case RangeCallback(callback) ⇒
          logger.trace(s"DatasetStorageClient.range() received server ask=$callback")
          callback
      })
      .multicast

    val clientError = MVar.empty[Throwable]

    /** Puts error to client error(for returning error to user of this client), and return reply with error for server.*/
    def handleClientErr(err: Throwable): F[RangeCallbackReply] = {
      run(
        clientError
          .put(ClientError(err.getMessage))
          .map(_ ⇒ RangeCallbackReply(RangeCallbackReply.Reply.ClientError(Error(err.getMessage))))
      )
    }

    val handleAsks = pullServerAsk
      .collect { case ask if ask.isDefined && !ask.isValue && !ask.isServerError ⇒ ask } // Collect callbacks
      .mapEval[F, RangeCallbackReply] {

        case ask if ask.isNextChildIndex ⇒
          val Some(nci) = ask.nextChildIndex

          rangeCallbacks
            .nextChildIndex(nci.keys.map(k ⇒ Key(k.toByteArray)).toArray, nci.childsChecksums.map(c ⇒ Hash(c.toByteArray)).toArray)
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
            .submitLeaf(sl.keys.map(k ⇒ Key(k.toByteArray)).toArray, sl.valuesChecksums.map(c ⇒ Hash(c.toByteArray)).toArray)
            .attempt
            .flatMap {
              case Left(err) ⇒
                handleClientErr(err)
              case Right(searchResult) ⇒
                Effect[F].pure(RangeCallbackReply(RangeCallbackReply.Reply.SubmitLeaf(ReplySubmitLeaf(
                  searchResult match {
                    case Searching.Found(i)          ⇒ ReplySubmitLeaf.SearchResult.Found(i)
                    case Searching.InsertionPoint(i) ⇒ ReplySubmitLeaf.SearchResult.InsertionPoint(i)
                  }
                ))))
            }

      }

    (
      Observable(
        RangeCallbackReply(
          RangeCallbackReply.Reply.DatasetId(DatasetId(ByteString.copyFrom(datasetId)))
        )
      ) ++ handleAsks
    ).subscribe(pushClientReply) // And clientReply response back to server

    pullServerAsk
      .collect {
        case ask if ask.isServerError ⇒
          val Some(err) = ask.serverError
          val serverError = ServerError(err.msg)
          // if server send an error we should close stream and lift error up
          Observable(pushClientReply.onError(serverError))
            .flatMap(_ ⇒ Observable.raiseError[(Array[Byte], Array[Byte])](serverError))
        case ask if ask.isValue ⇒
          val Some(RangeValue(key, value)) = ask._value
          Observable(key.toByteArray → value.toByteArray)

      }.flatten

  }

  /**
   * Initiates ''Put'' operation in remote MerkleBTree.
   *
   * @param datasetId      Dataset ID
   * @param putCallbacks   Wrapper for all callback needed for ''Put'' operation to the BTree.
   * @param encryptedValue Encrypted value.
   * @return returns old value if old value was overridden, None otherwise.
   */
  override def put(datasetId: Array[Byte], putCallbacks: BTreeRpc.PutCallbacks[F], encryptedValue: Array[Byte]): F[Option[Array[Byte]]] = {
    // Convert a remote stub call to monix pipe
    val pipe = callToPipe(stub.put)

    val (pushClientReply, pullServerAsk) = pipe
      .transform(_.map {
        case PutCallback(callback) ⇒
          logger.trace(s"DatasetStorageClient.put() received server ask=$callback")
          callback
      })
      .multicast

    val clientError = MVar.empty[Throwable]

    /** Puts error to client error(for returning error to user of this client), and return reply with error for server.*/
    def handleClientErr(err: Throwable): F[PutCallbackReply] = {
      run(
        clientError
          .put(ClientError(err.getMessage))
          .map(_ ⇒ PutCallbackReply(PutCallbackReply.Reply.ClientError(Error(err.getMessage))))
      )
    }

    val handleAsks = pullServerAsk
      .collect { case ask if ask.isDefined && !ask.isValue ⇒ ask } // Collect callbacks
      .mapEval[F, PutCallbackReply] {

        case ask if ask.isNextChildIndex ⇒
          val Some(nci) = ask.nextChildIndex

          putCallbacks
            .nextChildIndex(nci.keys.map(k ⇒ Key(k.toByteArray)).toArray, nci.childsChecksums.map(c ⇒ Hash(c.toByteArray)).toArray)
            .attempt
            .flatMap {
              case Left(err) ⇒
                handleClientErr(err)
              case Right(idx) ⇒
                Effect[F].pure(PutCallbackReply(PutCallbackReply.Reply.NextChildIndex(ReplyNextChildIndex(idx))))
            }

        case ask if ask.isPutDetails ⇒
          val Some(pd) = ask.putDetails

          putCallbacks
            .putDetails(pd.keys.map(k ⇒ Key(k.toByteArray)).toArray, pd.valuesChecksums.map(c ⇒ Hash(c.toByteArray)).toArray)
            .attempt
            .flatMap {
              case Left(err) ⇒
                handleClientErr(err)
              case Right(cpd) ⇒
                val putDetails = ReplyPutDetails(
                  key = ByteString.copyFrom(cpd.key.bytes),
                  checksum = ByteString.copyFrom(cpd.valChecksum.bytes),
                  searchResult = cpd.searchResult match {
                    case Searching.Found(foundIndex)              ⇒ ReplyPutDetails.SearchResult.Found(foundIndex)
                    case Searching.InsertionPoint(insertionPoint) ⇒ ReplyPutDetails.SearchResult.InsertionPoint(insertionPoint)
                  })
                Effect[F].pure(PutCallbackReply(PutCallbackReply.Reply.PutDetails(putDetails)))
            }

        case ask if ask.isVerifyChanges ⇒
          val Some(vc) = ask.verifyChanges

          putCallbacks
            .verifyChanges(Hash(vc.serverMerkleRoot.toByteArray), vc.splitted)
            .attempt
            .flatMap {
              case Left(err) ⇒
                handleClientErr(err)
              case Right(idx) ⇒
                Effect[F].pure(PutCallbackReply(PutCallbackReply.Reply.VerifyChanges(ReplyVerifyChanges()))) // TODO: here should be signature
            }

        case ask if ask.isChangesStored ⇒
          putCallbacks
            .changesStored()
            .attempt
            .flatMap {
              case Left(err) ⇒
                handleClientErr(err)
              case Right(idx) ⇒
                Effect[F].pure(PutCallbackReply(PutCallbackReply.Reply.ChangesStored(ReplyChangesStored())))
            }
      }

    (
      Observable( // Pass the datasetId and value as the first, unasked pushes
        PutCallbackReply(
          PutCallbackReply.Reply.DatasetId(DatasetId(ByteString.copyFrom(datasetId)))
        ),
        PutCallbackReply(
          PutCallbackReply.Reply.Value(PutValue(ByteString.copyFrom(encryptedValue)))
        )
      ) ++ handleAsks
    ).subscribe(pushClientReply) // And push response back to server

    val errorOrValue =
      pullServerAsk
        .collect { // Collect terminal task with value/error
          case ask if ask.isServerError ⇒
            val Some(err) = ask.serverError
            val serverError = ServerError(err.msg)
            // if server send the error we should close stream and lift error up
            Task(pushClientReply.onError(serverError))
              .flatMap(_ ⇒ Task.raiseError[Option[Array[Byte]]](serverError))
          case ask if ask.isValue ⇒
            val Some(getValue) = ask._value
            // if got success response or server error close stream and return value\error to user of this client
            Task(pushClientReply.onComplete())
              .map { _ ⇒
                Option(getValue.value)
                  .filterNot(_.isEmpty)
                  .map(_.toByteArray)
              }
        }.headOptionL // Take the first option value or server error

    run(errorOrValue.flatMap {
      case Some(value) ⇒ value // return success result or server error in first
      case None        ⇒ clientError.take.flatMap(err ⇒ Task.raiseError(err)) // return occurred clients error
    })

  }

  /**
   * Initiates ''Remove'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param removeCallbacks Wrapper for all callback needed for ''Remove'' operation to the BTree.
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  override def remove(datasetId: Array[Byte], removeCallbacks: BTreeRpc.RemoveCallback[F]): F[Option[Array[Byte]]] = ???

}

object DatasetStorageClient {

  /**
   * Shorthand to register [[DatasetStorageClient]] inside [[GrpcClient]].
   *
   * @param channel     Channel to remote node
   * @param callOptions Call options
   */
  def register[F[_] : Effect]()(
    channel: ManagedChannel,
    callOptions: CallOptions
  )(implicit scheduler: Scheduler): DatasetStorageRpc[F, Observable] =
    new DatasetStorageClient[F](new DatasetStorageRpcStub(channel, callOptions))

  /**  Error from server(node) side. */
  case class ServerError(msg: String) extends NoStackTrace {
    override def getMessage: String = msg
  }

}
