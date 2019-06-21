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

import cats.data.{EitherT, NonEmptyList}
import cats.effect.{IO, Timer}
import fluence.log.{Log, LogFactory}
import fluence.vm.VmError._
import fluence.vm.TestUtils._
import fluence.vm.wasm.MemoryHasher
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.ExecutionContext
import scala.language.implicitConversions

class WasmVmSpec extends WordSpec with Matchers {

  implicit def error[E](either: EitherT[IO, E, _]): E = either.value.unsafeRunSync().left.get

  private implicit val timer: Timer[IO] = IO.timer(ExecutionContext.global)
  private implicit val log: Log[IO] = LogFactory.forPrintln[IO]().init(getClass.getSimpleName).unsafeRunSync()

  "apply" should {

    "raise error" when {

      "config error" in {
        val res = for {
          vm <- WasmVm[IO](NonEmptyList.one("unknown file"), MemoryHasher[IO], "wrong config namespace")
        } yield vm

        val error = res.failed()
        error shouldBe a[InternalVmError]
        error.getMessage should startWith("Unable to read a config for the namespace")
      }

      "file not found" in {
        val res = for {
          vm <- WasmVm[IO](NonEmptyList.one("unknown file"), MemoryHasher[IO])
        } yield vm

        val error = res.failed()
        error shouldBe a[InitializationError]
        error.getMessage should startWith("Preparing execution context before execution was failed for")
      }

      // todo add more error cases with prepareContext and module initialization
      // (f.e. test case with two modules with the same module name - sum.wast and sum-copy.wast)
    }
  }

  "initialize Vm success" when {
    "with one file" in {
      val sumFile = getClass.getResource("/wast/sum.wast").getPath

      WasmVm[IO](NonEmptyList.one(sumFile), MemoryHasher[IO]).success()
    }

    "with two files with different module name" in {
      val sumFile = getClass.getResource("/wast/sum.wast").getPath
      val mulFile = getClass.getResource("/wast/mul.wast").getPath

      WasmVm[IO](NonEmptyList.of(mulFile, sumFile), MemoryHasher[IO]).success()
    }

    "two modules have function with the same names" in {
      // module without name and with some functions with the same name ("allocate", "deallocate", "invoke", ...)
      val sum1File = getClass.getResource("/wast/counter.wast").getPath
      // module without name and with some functions with the same name ("allocate", "deallocate", "invoke", ...)
      val sum2File = getClass.getResource("/wast/bad-allocation-function-i64.wast").getPath

      val res = for {
        vm <- WasmVm[IO](NonEmptyList.of(sum1File, sum2File), MemoryHasher[IO])
      } yield vm

      res.success()
    }

  }

}
