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
import cats.effect.IO
import org.scalatest.EitherValues

import scala.language.{higherKinds, implicitConversions}

// TODO: to run this test from IDE It needs to build vm-hello-world project explicitly at first
class HelloWorldIntegrationTest extends AppIntegrationTest with EitherValues {

  private val helloWorldFilePath =
    getModuleDirPrefix() + "/src/it/resources/test-cases/hello-world/target/wasm32-unknown-unknown/release/hello_world.wasm"

  "hello user app" should {

    "be able to instantiate" in {
      (for {
        vm ← WasmVm[IO](NonEmptyList.one(helloWorldFilePath), "fluence.vm.client.4Mb")
        state ← vm.getVmState[IO].toVmError

      } yield {
        state should not be None

      }).success()

    }

    "greets John correctly" in {
      (for {
        vm ← WasmVm[IO](NonEmptyList.one(helloWorldFilePath), "fluence.vm.client.4Mb")
        greetingResult ← vm.invoke[IO](None, "John".getBytes())
        state ← vm.getVmState[IO].toVmError

      } yield {
        checkTestResult(greetingResult, "Hello, world! From user John")
      }).success()

    }

    "operates correctly with empty input" in {
      (for {
        vm ← WasmVm[IO](NonEmptyList.one(helloWorldFilePath), "fluence.vm.client.4Mb")
        greetingResult ← vm.invoke[IO](None)
        state ← vm.getVmState[IO].toVmError

      } yield {
        checkTestResult(greetingResult, "Hello, world! From user")
      }).success()

    }

  }

}
