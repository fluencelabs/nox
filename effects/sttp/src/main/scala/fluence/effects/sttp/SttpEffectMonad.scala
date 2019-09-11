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
import com.softwaremill.sttp.{MonadError ⇒ ME}

import scala.language.higherKinds

class SttpEffectMonad[F[_]: Monad] extends ME[EitherT[F, SttpError, ?]] {
  override def unit[T](t: T): EitherT[F, SttpError, T] = EitherT.pure[F, SttpError](t)

  override def map[T, T2](fa: EitherT[F, SttpError, T])(f: T ⇒ T2): EitherT[F, SttpError, T2] = fa.map(f)

  override def flatMap[T, T2](fa: EitherT[F, SttpError, T])(
    f: T ⇒ EitherT[F, SttpError, T2]
  ): EitherT[F, SttpError, T2] = fa.flatMap(f)

  override def error[T](t: Throwable): EitherT[F, SttpError, T] =
    EitherT.leftT[F, T](SttpRequestError(t))

  override protected def handleWrappedError[T](
    rt: EitherT[F, SttpError, T]
  )(h: PartialFunction[Throwable, EitherT[F, SttpError, T]]): EitherT[F, SttpError, T] =
    rt.leftFlatMap {
      case SttpRequestError(e) if h.isDefinedAt(e) ⇒ h(e)
      case e ⇒ EitherT.leftT[F, T](e)
    }
}
