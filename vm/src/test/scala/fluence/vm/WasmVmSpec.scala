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
import cats.effect.IO
import fluence.vm.VmError._
import fluence.vm.TestUtils._
import org.scalatest.{Matchers, WordSpec}

import scala.language.implicitConversions

class WasmVmSpec extends WordSpec with Matchers {

  implicit def error[E](either: EitherT[IO, E, _]): E = either.value.unsafeRunSync().left.get

  "apply" should {

    "raise error" when {

      "config error" in {
        val res = for {
          vm <- WasmVm[IO](NonEmptyList.one("unknown file"), "wrong config namespace")
        } yield vm

        val error = res.failed()
        error shouldBe a[InternalVmError]
        error.getMessage should startWith("Unable to read a config for the namespace")
      }

      "file not found" in {
        val res = for {
          vm <- WasmVm[IO](NonEmptyList.one("unknown file"))
        } yield vm

        val error = res.failed()
        error shouldBe a[InitializationError]
        error.getMessage should startWith("Preparing execution context before execution was failed for")
      }

      "2 module has function with equal name" in {
        // module without name and with some functions with the same name ("allocate", "deallocate", "test", ...)
        val sum1File = getClass.getResource("/wast/no-getMemory.wast").getPath
        // module without name and with some functions with the same name ("allocate", "deallocate", "test", ...)
        val sum2File = getClass.getResource("/wast/bad-allocation-function.wast").getPath

        val res = for {
          vm <- WasmVm[IO](NonEmptyList.of(sum1File, sum2File))
        } yield vm

        val error = res.failed()
        error shouldBe a[InitializationError]
        error.getMessage should endWith("was already registered")
      }

      // todo add more error cases with prepareContext and module initialization
      // (f.e. test case with two modules with the same module name - sum.wast and sum-copy.wast)
    }
  }

  "initialize Vm success" when {
    "there is one file" in {
      val sumFile = getClass.getResource("/wast/sum.wast").getPath

      WasmVm[IO](NonEmptyList.one(sumFile)).success()
    }

    "there are two files" in {
      val sumFile = getClass.getResource("/wast/sum.wast").getPath
      val mulFile = getClass.getResource("/wast/mul.wast").getPath

      WasmVm[IO](NonEmptyList.of(mulFile, sumFile)).success()
    }

  }

}
