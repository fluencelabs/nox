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

package fluence.kad.http.dht

import cats.effect.Sync
import cats.syntax.flatMap._
import cats.syntax.apply._
import fluence.codec.PureCodec
import fluence.effects.kvstore.KVStore
import fluence.kad.http.KeyDecoder
import fluence.kad.protocol.Key
import fluence.log.LogFactory
import org.http4s.HttpRoutes
import org.http4s.dsl.Http4sDsl

import scala.language.higherKinds

class DhtHttp[F[_]: Sync](
  prefix: String,
  store: KVStore[F, Key, Array[Byte]]
) {
  import KeyDecoder._

  def routes()(implicit dsl: Http4sDsl[F], lf: LogFactory[F]): HttpRoutes[F] = {
    import dsl._

    HttpRoutes.of[F] {
      case GET -> Root / `prefix` / KeyVar(key) ⇒
        LogFactory[F].init("dht-http" -> s"$prefix/get", "key" -> key.asBase58) >>= { implicit log ⇒
          store.get(key).value.flatMap {
            case Left(err) ⇒
              log.error("Getting data errored", err) *>
                InternalServerError(err.getMessage)
            case Right(None) ⇒
              NotFound()
            case Right(Some(value)) ⇒
              Ok(value)
          }
        }

      case req @ PUT -> Root / `prefix` / KeyVar(key) ⇒
        LogFactory[F].init("dht-http" -> s"$prefix/put", "key" -> key.asBase58) >>= { implicit log ⇒
          req.body.compile
            .foldChunks(Array.emptyByteArray) {
              case (acc, chunk) ⇒
                // TODO optimize
                Array.concat(acc, chunk.toArray)
            }
          // TODO check authentication somehow?
            .flatMap(store.put(key, _).value)
            .flatMap {
              case Right(_) ⇒
                NoContent()
              case Left(err) ⇒
                log.error("Storing data errored", err) *>
                  BadRequest(err.getMessage)
            }
        }
    }
  }

}

object DhtHttp {

  def apply[F[_]: Sync, V](prefix: String,
                           store: KVStore[F, Key, V])(implicit valueCodec: PureCodec[V, Array[Byte]]): DhtHttp[F] =
    new DhtHttp[F](prefix, store.transformValues[Array[Byte]])

}
