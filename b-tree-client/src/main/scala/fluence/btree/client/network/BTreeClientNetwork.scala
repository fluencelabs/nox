package fluence.btree.client.network

import cats.MonadError
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.btree.client.Value
import fluence.btree.client.network.BTreeClientNetwork.{ GetCallback, PutCallback }

/**
 * Base network abstraction for interaction BTree client with BTree server.
 * Under the hood makes a series of round trip requests between this client and the server.
 *
 * @param ME Monad error instance for the effect ''F''
 * @tparam F The type of effect, box for returning value
 * @tparam K The type of plain text ''key''
 * @tparam V The type of plain text ''value''
 */
abstract class BTreeClientNetwork[F[_], K, V](implicit ME: MonadError[F, Throwable]) {

  /**
   * Makes '''Get''' request to the Btree server.
   *
   * @param state       State of get request
   * @param onResponse Callback on server response. Will invoked on each server response for ''get'' request.
   */
  def get(state: GetState[K], onResponse: GetCallback[F, K]): F[Option[Value]] = {

    for {
      reqRes ← doRequest(state)
      callbackRes ← onResponse(reqRes)
      searchResult ← callbackRes match {
        case Left(newRequest) ⇒ get(newRequest, onResponse)
        case Right(result)    ⇒ ME.pure(result)
      }
    } yield searchResult

  }

  /**
   * Makes '''Put''' request to the Btree server.
   *
   * @param state       State of 'put' request
   * @param onResponse Callback on server response. Will invoked on each server response for ''put'' request.
   */
  def put(state: PutState[K, V], onResponse: PutCallback[F, K, V]): F[Option[Value]] = {

    for {
      reqRes ← doRequest(state)
      callbackRes ← onResponse(reqRes)
      searchResult ← callbackRes match {
        case Left(newRequest) ⇒ put(newRequest, onResponse)
        case Right(result)    ⇒ ME.pure(result)
      }
    } yield searchResult

  }

  /**
   * Does specified request to the server.
   *
   * @param state Client request to be executed
   * @return Tuple with specified client request and received server response
   */
  def doRequest(state: RequestState): F[(RequestState, BTreeServerResponse)]

}

object BTreeClientNetwork {

  type GetCallback[F[_], K] = ((RequestState, BTreeServerResponse)) ⇒ F[Either[GetState[K], Option[Value]]]

  type PutCallback[F[_], K, V] = ((RequestState, BTreeServerResponse)) ⇒ F[Either[PutState[K, V], Option[Value]]]

}
