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
import cats.effect.{IO, Resource}
import org.scalatest.{Matchers, WordSpec}
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

trait KVStoreTestKit extends WordSpec with Matchers {
  def storeMaker: Resource[IO, KVStore[IO, Long, String]]

  "store" should {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(true, true, true)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.DEBUG

    "save and load" in {
      storeMaker.use { store ⇒
        IO {

          store.put(12312323l, "value").value.unsafeRunSync().isRight shouldBe true
          store.get(12312323l).value.unsafeRunSync() shouldBe Right(Some("value"))

          store.stream.compile.toList.unsafeRunSync() shouldBe ((12312323l, "value") :: Nil)

          store.remove(12312323l).value.unsafeRunSync().isRight shouldBe true

          store.get(12312323l).value.unsafeRunSync() shouldBe Right(None)

          store.stream.compile.toList.unsafeRunSync() shouldBe empty

        }
      }.unsafeRunSync()
    }
  }
}
