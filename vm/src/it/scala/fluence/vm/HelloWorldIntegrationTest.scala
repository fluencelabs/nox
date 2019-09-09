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

// TODO: to run this test from IDE It needs to build vm-hello-world project explicitly at first
class HelloWorldIntegrationTest extends AppIntegrationTest with EitherValues {

  private implicit val timer: Timer[IO] = IO.timer(ExecutionContext.global)
  private implicit val log: Log[IO] = LogFactory.forPrintln[IO]().init(getClass.getSimpleName).unsafeRunSync()

  private val helloWorldFilePath =
    getModuleDirPrefix() + "/Users/trofim/Desktop/work/fluence/fluid/backend-c/hello_world.wasm"

  "hello user app" should {

    "be able to instantiate" in {
      (for {
        vm ← WasmVm[IO](NonEmptyList.one(helloWorldFilePath), MemoryHasher[IO], "fluence.vm.client.4Mb")
        state ← vm.getVmState[IO].toVmError

      } yield {
        state should not be None

      }).success()

    }

    "greets John correctly" in {
        val res = for {
        vm ← WasmVm[IO](NonEmptyList.fromList("/Users/trofim/Desktop/work/fluence/sqlite/sqlite3.wasm" :: "/Users/trofim/Desktop/work/fluence/fluid/backend-c/fluid.wasm" :: Nil).get, MemoryHasher[IO], "fluence.vm.client.2Gb")
        res1 ← vm.invoke[IO]("{\"action\":\"Post\",\"message\":\"I'm nice, you're nice, it's nice!\",\"username\":\"random_joe\"}".getBytes())
        res2 ← vm.invoke[IO]("{\"action\" : \"Post\", \"username\" : \"a\", \"message\": \"1111\"}".getBytes())
        res2 ← vm.invoke[IO]("{\"action\" : \"Post\", \"username\" : \"b\", \"message\": \"1111\"}".getBytes())
        res2 ← vm.invoke[IO]("{\"action\" : \"Post\", \"username\" : \"asdasd\", \"message\": \"aaa\"}".getBytes())
        res2 ← vm.invoke[IO]("{\"action\" : \"Fetch\", \"username\" : \"asdasd\"}".getBytes())
        res3 ← vm.invoke[IO]("{\"action\" : \"Fetch\", \"username\" : \"random_joe\"}".getBytes())
        _ ← vm.getVmState[IO].toVmError

        _ = System.out.println(new String(res1.output))
        _ = System.out.println(new String(res1.output))
        _ = System.out.println(new String(res2.output))
        _ = System.out.println(new String(res3.output))
      } yield {
        checkTestResult(res2, "Hello, world! From user John")
      }

      val tt = res.failed()
      val yy = tt

    }

    "operates correctly with empty input" in {
      (for {
        vm ← WasmVm[IO](NonEmptyList.one(helloWorldFilePath), MemoryHasher[IO], "fluence.vm.client.4Mb")
        greetingResult ← vm.invoke[IO]()
        _ ← vm.getVmState[IO].toVmError

      } yield {
        checkTestResult(greetingResult, "Hello, world! From user")
      }).success()

    }

  }

}
