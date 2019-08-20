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

import cats.data.NonEmptyList
import cats.effect.{IO, Timer}
import fluence.log.{Log, LogFactory}
import fluence.vm.wasm.MemoryHasher
import org.scalatest.EitherValues

import scala.concurrent.ExecutionContext
import scala.language.{higherKinds, implicitConversions}

// TODO: to run this test from IDE It needs to build vm-counter project explicitly at first
class CounterIntegrationTest extends AppIntegrationTest with EitherValues {

  private implicit val timer: Timer[IO] = IO.timer(ExecutionContext.global)
  private implicit val log: Log[IO] = LogFactory.forPrintln[IO]().init(getClass.getSimpleName).unsafeRunSync()

  private val counterFilePath =
    getModuleDirPrefix() + "/src/it/resources/test-cases/counter/target/wasm32-unknown-unknown/release/counter.wasm"

  "counter app" should {

    "be able to instantiate" in {
      (for {
        vm ← WasmVm[IO](NonEmptyList.one(counterFilePath), MemoryHasher[IO])
        state ← vm.getVmState[IO].toVmError

      } yield {
        state should not be None

      }).success()

    }

    "increment counter and returns its state" in {
      (for {
        vm ← WasmVm[IO](NonEmptyList.one(counterFilePath), MemoryHasher[IO])
        _ ← vm.invoke[IO]()
        getResult1 ← vm.invoke[IO]()
        _ ← vm.invoke[IO]()
        getResult2 ← vm.invoke[IO]()
        _ ← vm.getVmState[IO].toVmError

      } yield {
        compareArrays(getResult1.output, Array[Byte](2, 0, 0, 0, 0, 0, 0, 0))
        compareArrays(getResult2.output, Array[Byte](4, 0, 0, 0, 0, 0, 0, 0))
      }).success()

    }

  }

}
