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

import java.io.File
import java.nio.ByteBuffer
import java.nio.file.{Files, Path, Paths}

import cats.data.EitherT
import cats.effect.{ContextShift, IO, Timer}
import com.softwaremill.sttp.{MonadError => _, _}

import scala.language.{higherKinds, implicitConversions}
import com.softwaremill.sttp.SttpBackend
import fluence.EitherTSttpBackend
import fs2.RaiseThrowable
import io.circe.fs2.stringStreamParser
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global

/**
 * Example to check IPFS interaction.
 */
object IpfsExample extends App {
  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  implicit val sttp: SttpBackend[EitherT[IO, Throwable, ?], fs2.Stream[IO, ByteBuffer]] = EitherTSttpBackend[IO]()


  implicit val rt = new RaiseThrowable[fs2.Pure] {}
  val k = fs2.Stream.emit("incorrect json").through(stringStreamParser[fs2.Pure]).attempt.toList
  println(k)

  val data = ByteVector(Array[Byte](1, 2, 3, 4))
  val store = new IpfsClient[IO](uri"http://data.fluence.one:5001")

  val home = System.getProperty( "user.home" )

  val hashE = store.upload(data).value.unsafeRunSync()
  println(hashE)

  val dirHash = store.upload(Paths.get(home).resolve("testdir")).value.unsafeRunSync().right.get
  println("dir upload: " + dirHash)
  println("file upload: " + store.upload(Paths.get(home).resolve("test.wasm")).value.unsafeRunSync())
  println("empty upload: " + store.upload(Paths.get(home).resolve("emptydir")).value.unsafeRunSync())


  val hash1 = hashE.right.get

  val res1 = store
    .download(hash1)
    .value
    .unsafeRunSync()
    .right
    .get
    .collect {
      case bb =>
        ByteVector(bb)
    }
    .compile
    .toList
    .unsafeRunSync()
  println("file response = " + res1.head.toArray.mkString(" "))

  val res2 = store
    .ls(dirHash)
    .value
    .unsafeRunSync()
  println("directory response = " + res2)

  sys.exit(0)
}
