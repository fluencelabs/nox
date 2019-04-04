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

package fluence.effects.ipfs

import java.nio.ByteBuffer

import cats.data.EitherT
import cats.effect.{ContextShift, IO, Timer}
import com.softwaremill.sttp.{MonadError => _, _}

import scala.language.{higherKinds, implicitConversions}
import com.softwaremill.sttp.SttpBackend
import fluence.EitherTSttpBackend
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global

object TestMain extends App {
  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  implicit val sttp: SttpBackend[EitherT[IO, Throwable, ?], fs2.Stream[IO, ByteBuffer]] = EitherTSttpBackend[IO]()

  val store = new IpfsStore[IO](uri"http://data.fluence.one:5001")

  val res1 = store
    .fetch(ByteVector.fromValidHex("0x73d0cc44bdac8f7f3d3893d5a30448d73d1bf686aec138ebdb9527b8c6d22779"))
    .value
    .unsafeRunSync()
    .right
    .get
    .collect {
      case bb =>
        val a = ByteVector(bb).decodeUtf8
        println(a)
        a
    }
    .compile
    .drain
    .unsafeRunSync()
  println("file response = " + res1)

  val res2 = store
    .ls(ByteVector.fromValidHex("0x5f220381bf07054f4f1ee0b454b9427ac9b1b79d145ab30004ea8a37f1a64157"))
    .value
    .unsafeRunSync()
  println("directory response = " + res2)

  sys.exit(0)
}
