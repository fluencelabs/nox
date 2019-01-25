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

// TODO: to run this test from IDE It needs to build vm-hello-user project explicitly at first
class HelloUserIntegrationTest extends AppIntegrationTest with EitherValues {

  private val helloUserFilePath =
    getModuleDirPrefix() + "/examples/hello-user/target/wasm32-unknown-unknown/release/hello_user.wasm"

  "hello user app" should {

    "be able to instantiate" in {
      (for {
        vm ← WasmVm[IO](NonEmptyList.one(helloUserFilePath))
        state ← vm.getVmState[IO].toVmError

      } yield {
        state should not be None

      }).success()

    }

    "greeting correctly" in {
      (for {
        vm ← WasmVm[IO](NonEmptyList.one(helloUserFilePath))
        greetingResult ← vm.invoke[IO](None, "John".getBytes())
        state ← vm.getVmState[IO].toVmError

      } yield {
        checkTestResult(greetingResult, "Hello John from Fluence")
      }).success()

    }

  }

}
