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

import cats.{Monad, MonadError}
import cats.effect._
import cats.effect.concurrent.{MVar, Ref}
import cats.effect.syntax.bracket._
import cats.syntax.apply._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.vm.WasmVm
import org.http4s.Response
import org.http4s.dsl.Http4sDsl
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.util.Try

/**
 * Representation of /apps/N/tx request, sends a tx to WasmVM
 */
case class Tx(appId: Long, path: String, body: String)

/**
 * Representation of /apps/N/query request, reads result
 */
case class Query(appId: Long, path: String)

/**
 * Tx ordering
 */
case class TxId(session: String, count: Int)

/**
 * Wrapper around WasmVM
 * - Saves results from WasmVM to memory
 * - Keeps txs in order
 * - Locks shared data with a mutex
 *
 * @param vm        WasmVM
 * @param responses Map containing responses
 * @param mutex     Lock for responses map
 * @param order     Txs ordering mechanism
 */
case class TxProcessor[F[_]: Sync: Monad: LiftIO] private (
  vm: WasmVm,
  responses: Ref[F, Map[String, String]],
  mutex: MVar[F, Unit],
  order: TxOrder[F]
)(
  implicit dsl: Http4sDsl[F]
) {

  import dsl._

  // unsafe!!!
  private def getId(path: String): F[TxId] =
    Sync[F].fromEither(path.split("/") match {
      case Array(session: String, count: String) =>
        Try(count.toInt).toEither
          .leftMap(e => new RuntimeException(s"Error parsing count: ${e.getMessage}", e))
          .map(TxId(session, _))
      case _ => Either.left(new RuntimeException("Wrong tx path format, expected session/count, got: " + path))
    })

  private def acquire(): F[Unit] = mutex.put(())

  private def release(): F[Unit] = mutex.take

  private def locked[A](thunk: F[A]): F[A] = acquire().bracket(_ => thunk)(_ => release())

  def processTx(tx: Tx): F[Response[F]] = {
    import tx._

    val waitOrder = getId(tx.path) >>= order.waitOrder
    val setLastId = getId(tx.path) >>= order.set

    waitOrder *> locked {
      for {
        result <- vm.invoke[F](None, body.getBytes()).value.flatMap(Sync[F].fromEither)
        encoded = ByteVector(result).toBase64
        _ <- responses.update(_.updated(path, encoded))
        json = s"""
                  | {
                  |  "jsonrpc": "2.0",
                  |  "id": "dontcare",
                  |  "result": {
                  |    "code": 0,
                  |    "data": "$encoded",
                  |    "hash": "no hash"
                  |  }
                  | }
            """.stripMargin
        response <- Ok(json)
      } yield response
    } <* setLastId
  }

  def processQuery(query: Query): F[Response[F]] = {
    import query._

    for {
      result <- responses.get.map(_.get(path)).map(_.getOrElse("not found"))
      json = s"""
                | {
                |   "jsonrpc": "2.0",
                |   "id": "dontcare",
                |   "result": {
                |     "response": {
                |       "info": "Responded for path $path",
                |       "value": "$result"
                |     }
                |   }
                | }
           """.stripMargin
      response <- Ok(json)
    } yield response
  }
}

object TxProcessor {

  import cats.syntax.flatMap._
  import cats.syntax.functor._

  type ME[F[_]] = MonadError[F, Throwable]

  def apply[F[_]: ContextShift: Concurrent: Timer: Sync: ME](vm: WasmVm)(
    implicit dsl: Http4sDsl[F]
  ): F[TxProcessor[F]] = {
    for {
      map <- Ref.of[F, Map[String, String]](Map.empty[String, String])
      mutex <- MVar.empty[F, Unit]
      ref <- Ref[F].of(Map.empty[String, Int])
      order = TxOrder(ref)
    } yield TxProcessor(vm, map, mutex, order)

  }
}
