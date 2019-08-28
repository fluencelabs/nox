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

package fluence.effects.sttp

import cats.Monad
import cats.data.EitherT
import com.softwaremill.sttp.Response

import scala.language.higherKinds

package object syntax {
  implicit class ResponseFOps[F[_]: Monad, T](resp: EitherT[F, SttpError,Response[T]]) {
    def toBody: EitherT[F, SttpError, T] =
      toBodyE(SttpBodyError)

    def toBodyE[E](fn: String ⇒ E): EitherT[F, E, T] =
      resp.subflatMap[E, T](_.body.left.map(fn))

    def decodeBody[TT](fn: T ⇒ Either[Throwable, TT]): EitherT[F, SttpError, TT] =
      toBody.subflatMap(fn(_).left.map(SttpDecodeError))
  }
}
