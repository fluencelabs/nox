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

import cats.data.EitherT
import cats.effect.{Async, ConcurrentEffect, ContextShift, Resource, Sync}
import cats.syntax.functor._
import cats.{~>, ApplicativeError, Functor}
import cats.syntax.applicativeError._
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import com.softwaremill.sttp.asynchttpclient.fs2.AsyncHttpClientFs2Backend
import com.softwaremill.sttp.impl.cats.implicits._
import com.softwaremill.sttp.{MonadError ⇒ ME}

import scala.language.higherKinds

object SttpEffect {

  private def eitherTArrow[F[_]: Functor](implicit ae: ApplicativeError[F, Throwable]): F ~> EitherT[F, SttpError, ?] =
    new (F ~> EitherT[F, SttpError, ?]) {
      override def apply[A](fa: F[A]): EitherT[F, SttpError, A] =
        EitherT(fa.attempt.map(_.left.map(SttpRequestError)))
    }

  def stream[F[_]: ConcurrentEffect]: SttpStreamEffect[F] =
    AsyncHttpClientFs2Backend[F]()
      .mapK(eitherTArrow[F])(new SttpEffectMonad[F])

  def plain[F[_]: Async: ContextShift]: SttpEffect[F] =
    sttpBackendToCatsMappableSttpBackend[F, Nothing](AsyncHttpClientCatsBackend[F]())
      .mapK(eitherTArrow[F])(new SttpEffectMonad[F])

  def streamResource[F[_]: ConcurrentEffect]: Resource[F, SttpStreamEffect[F]] =
    Resource.make(Sync[F].delay(stream[F]))(sb ⇒ Sync[F].delay(sb.close()))

  def plainResource[F[_]: Async: ContextShift]: Resource[F, SttpEffect[F]] =
    Resource.make(Sync[F].delay(plain[F]))(sb ⇒ Sync[F].delay(sb.close()))
}
