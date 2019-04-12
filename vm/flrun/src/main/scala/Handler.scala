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

import cats.effect.IO
import cats.effect.concurrent.{MVar, Ref}
import cats.syntax.apply._
import fluence.vm.WasmVm
import org.http4s.Response
import org.http4s.dsl.Http4sDsl
import scodec.bits.ByteVector

case class Tx(appId: Long, path: String, body: String)

case class Query(appId: Long, path: String)

case class Handler(vm: WasmVm, map: Ref[IO, Map[String, String]], mutex: MVar[IO, Unit], order: Order)(
  implicit dsl: Http4sDsl[IO]
) {

  import dsl._

  // unsafe!!!
  private def getId(path: String) = path.split("/").last.toInt

  private def acquire(): IO[Unit] = mutex.put(())

  private def release(): IO[Unit] = mutex.take

  private def locked[A](io: IO[A]): IO[A] = acquire().bracket(_ => io)(_ => release())

  def processTx(tx: Tx): IO[Response[IO]] = {
    import tx._

    order.wait(getId(tx.path)) *> locked {
      for {
        result <- vm.invoke[IO](None, body.getBytes()).value.flatMap(IO.fromEither)
        encoded = ByteVector(result).toBase64
        _ <- map.update(_.updated(path, encoded))
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
    } <* order.set(getId(tx.path))
  }

  def processQuery(query: Query): IO[Response[IO]] = {
    import query._

    for {
      result <- map.get.map(_.get(path)).map(_.getOrElse("not found"))
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
