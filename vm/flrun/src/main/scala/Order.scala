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

import cats.effect.concurrent.Ref
import cats.effect.{ContextShift, IO, Timer}
import cats.syntax.apply._

import scala.concurrent.duration._

case class Order(ref: Ref[IO, Int])(implicit cs: ContextShift[IO], t: Timer[IO]) {

  def wait(id: Int): IO[Unit] =
    for {
      last <- ref.get
      _ <- if (id - last == 1 || last < 0) {
        IO.unit
      } else {
        IO.shift *> IO.sleep(100.millis) *> wait(id)
      }
    } yield ()

  def set(id: Int): IO[Unit] =
    ref.set(id)
}
