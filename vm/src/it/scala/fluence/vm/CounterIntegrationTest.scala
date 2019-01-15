package fluence.vm

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

import cats.data.NonEmptyList
import cats.effect.IO
import org.scalatest.EitherValues

import scala.language.{higherKinds, implicitConversions}

// TODO: to run this test from IDE It needs to build vm-counter project explicitly at first
class CounterIntegrationTest extends AppIntegrationTest with EitherValues {

  private val counterFilePath =
    getModuleDirPrefix() + "/examples/counter/target/wasm32-unknown-unknown/release/counter.wasm"

  "counter app" should {

    "be able to instantiate" in {
      (for {
        vm <- WasmVm[IO](NonEmptyList.one(counterFilePath))
        state <- vm.getVmState[IO].toVmError

      } yield {
        state should not be None

      }).success()

    }

    "increment counter and returns its state" in {
      (for {
        vm <- WasmVm[IO](NonEmptyList.one(counterFilePath))
        _ <- vm.invoke[IO]()
        getResult1 <- vm.invoke[IO]()
        _ <- vm.invoke[IO]()
        getResult2 <- vm.invoke[IO]()
        _ <- vm.getVmState[IO].toVmError

      } yield {
        checkTestResult(getResult1, "1")
        checkTestResult(getResult2, "3")
      }).success()

    }

  }

}
