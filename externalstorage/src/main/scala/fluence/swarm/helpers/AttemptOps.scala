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

package fluence.swarm.helpers
import cats.Applicative
import cats.data.EitherT
import scodec.{Attempt, Err}

import scala.language.higherKinds

object AttemptOps {
  implicit class RichAttempt[T](attempt: Attempt[T]) {
    def toEitherT[F[_]: Applicative]: EitherT[F, Err, T] = EitherT.fromEither(attempt.toEither)
    def toEitherT[F[_]: Applicative, E](errFunc: Err => E): EitherT[F, E, T] = toEitherT.leftMap(errFunc)
  }
}
