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

package fluence.vm

import cats.effect.IO
import org.scalatest.{EitherValues, Matchers, OptionValues, WordSpec}

import scala.language.{higherKinds, implicitConversions}

// TODO: now for run this test from IDE It is needed to build vm-counter project explicitly
class CounterIntegrationTest extends WordSpec with Matchers with EitherValues with OptionValues {

  val moduleDirPrefix =
    if (System.getProperty("user.dir").endsWith("/vm"))
      System.getProperty("user.dir")
    else
      System.getProperty("user.dir") + "/vm/"

  val counterFilePath =
    moduleDirPrefix + "/examples/counter/target/wasm32-unknown-unknown/release/counter.wasm"

  "counter example" should {

    "be able to instantiate" in {
      (for {
        vm <- WasmVm[IO](Seq(counterFilePath))
        state <- vm.getVmState[IO].toVmError
      } yield {
        state should not be None

      }).value.unsafeRunSync().right.value

    }

    "increment counter and returns its state" in {

      (for {
        vm <- WasmVm[IO](Seq(counterFilePath))
        _ <- vm.invoke[IO](None, "inc")
        getResult1 <- vm.invoke[IO](None, "get")
        _ <- vm.invoke[IO](None, "inc")
        _ <- vm.invoke[IO](None, "inc")
        getResult2 <- vm.invoke[IO](None, "get")
        _ <- vm.getVmState[IO].toVmError

      } yield {
        getResult1 shouldBe defined

        val resultAsString = new String(getResult1.value)
        resultAsString equals "1"
      }).value.unsafeRunSync.right.value

    }

  }

}
