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

package fluence.effects.kvstore

import java.nio.file.Files

import cats.effect.{ContextShift, IO, Resource, Timer}
import cats.syntax.compose._
import fluence.codec.PureCodec
import fluence.log.{Log, LogFactory}

import scala.concurrent.ExecutionContext.Implicits.global

class RocksDBStoreSpec extends KVStoreTestKit {
  implicit val shift: ContextShift[IO] = IO.contextShift(global)

  implicit val timer: Timer[IO] = IO.timer(global)

  implicit val stringCodec: PureCodec[String, Array[Byte]] =
    PureCodec.liftB(_.getBytes(), bs ⇒ new String(bs))

  implicit val longCodec: PureCodec[Array[Byte], Long] =
    PureCodec[Array[Byte], String] andThen PureCodec
      .liftB[String, Long](_.toLong, _.toString)

  override implicit val log: Log[IO] = LogFactory.forPrintln[IO]().init("rocksDbSpec").unsafeRunSync()

  override def storeMaker: Resource[IO, KVStore[IO, Long, String]] =
    RocksDBStore.make[IO, Long, String](Files.createTempDirectory("rocksdbspec").toString)
}
