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

import cats.Monad
import cats.data.EitherT
import cats.effect.IO
import fluence.crypto.{Crypto, CryptoError, DumbCrypto}
import fluence.vm.TestUtils._
import fluence.vm.VmError._
import org.scalatest.{Matchers, WordSpec}

import scala.language.{higherKinds, implicitConversions}

class AsmleWasmVmSpec extends WordSpec with Matchers {

  "invoke" should {
    "raise error" when {

      "unable to find a function" in {
        val sumFile = getClass.getResource("/wast/sum.wast").getPath

        val res = for {
          vm <- WasmVm[IO](Seq(sumFile))
          result ← vm.invoke[IO](None, "wrongFnName", Seq("100", "13", "extraArg")).toVmError
        } yield result
        val error = res.failed()
        error shouldBe a[NoSuchFnError]
        error.getMessage should startWith("Unable to find a function with the name='<no-name>.wrongFnName'")
      }

      "invalid number of arguments" in {
        val sumFile = getClass.getResource("/wast/sum.wast").getPath
        val res = for {
          vm <- WasmVm[IO](Seq(sumFile))
          result ← vm.invoke[IO](Some("SumModule"), "sum", Seq("100", "13", "extraArg")).toVmError
        } yield result
        val error = res.failed()
        error shouldBe a[InvalidArgError]
        error.getMessage should startWith(
          "Invalid number of arguments, expected=2, actually=3 for fn='SumModule.sum'"
        )
      }

      "invalid type for arguments" in {
        val sumFile = getClass.getResource("/wast/sum.wast").getPath
        val res = for {
          vm <- WasmVm[IO](Seq(sumFile))
          result ← vm.invoke[IO](Some("SumModule"), "sum", Seq("stringParam", "[1, 2, 3]")).toVmError
        } yield result
        val error = res.failed()
        error shouldBe a[InvalidArgError]
        error.getMessage should startWith("Arg 0 of 'stringParam' not an int")
      }

      "wasm code fell into the trap" in {
        val sumFile = getClass.getResource("/wast/sum-with-trap.wast").getPath
        val res = for {
          vm <- WasmVm[IO](Seq(sumFile))
          result ← vm.invoke[IO](None, "sum", Seq("100", "13")).toVmError // Integer overflow
        } yield result
        val error = res.failed()
        error shouldBe a[TrapError]
        error.getMessage should startWith("Function '<no-name>.sum' with args: List(100, 13) was failed")
      }

    }
  }

  "invokes function success" when {
    "run sum.wast" in {
      val sumFile = getClass.getResource("/wast/sum.wast").getPath

      val res = for {
        vm <- WasmVm[IO](Seq(sumFile))
        result ← vm.invoke[IO](Some("SumModule"), "sum", Seq("100", "13")).toVmError
      } yield {
        result should not be None
        result.get.deep shouldBe Array[Byte](113, 0, 0, 0).deep
      }

      res.success()
    }

    "run sum.wast and after that mul.wast" in {
      val sumFile = getClass.getResource("/wast/sum.wast").getPath
      val mulFile = getClass.getResource("/wast/mul.wast").getPath

      val res = for {
        vm <- WasmVm[IO](Seq(mulFile, sumFile))
        mulResult ← vm.invoke[IO](Some("MulModule"), "mul", Seq("100", "13"))
        sumResult ← vm.invoke[IO](Some("SumModule"), "sum", Seq("100", "13")).toVmError
      } yield {
        mulResult should not be None
        sumResult should not be None

        mulResult.get.deep shouldBe Array[Byte](20, 5, 0, 0).deep
        sumResult.get.deep shouldBe Array[Byte](113, 0, 0, 0).deep
      }

      res.success()
    }

    "run counter.wast" in {
      val file = getClass.getResource("/wast/counter.wast").getPath

      val res = for {
        vm <- WasmVm[IO](Seq(file))
        get0 ← vm.invoke[IO](None, "get", Nil) // read 0
        _ ← vm.invoke[IO](None, "inc", Nil) // 0 -> 1
        get1 ← vm.invoke[IO](None, "get", Nil) // read 1
        _ ← vm.invoke[IO](None, "inc", Nil) // 1 -> 2
        _ ← vm.invoke[IO](None, "inc", Nil) // 2 -> 3
        get2 ← vm.invoke[IO](None, "get", Nil).toVmError //read 3
      } yield {
        get0 should not be None
        get1 should not be None
        get2 should not be None

        get0.get.deep shouldBe Array[Byte](0, 0, 0, 0).deep
        get1.get.deep shouldBe Array[Byte](1, 0, 0, 0).deep
        get2.get.deep shouldBe Array[Byte](3, 0, 0, 0).deep
      }

      res.success()
    }

  }

  "getVmState" should {
    "raise an error" when {
      "getting inner state for module failed" in {
        val counterFile = getClass.getResource("/wast/counter.wast").getPath
        val badHasher = new Crypto.Hasher[Array[Byte], Array[Byte]] {
          override def apply[F[_]: Monad](input: Array[Byte]): EitherT[F, CryptoError, Array[Byte]] =
            EitherT.leftT(CryptoError("error!"))
        }
        val res = for {
          vm <- WasmVm[IO](Seq(counterFile), cryptoHasher = badHasher)
          state ← vm.getVmState[IO].toVmError
        } yield state

        val error = res.failed()
        error.getMessage shouldBe "Getting internal state for module=<no-name> failed"
        error.getCause shouldBe a[CryptoError]
        error shouldBe a[InternalVmError]
      }

    }

    "returns state" when {
      "there is one module without memory present" in {
        val testHasher = DumbCrypto.testHasher
        // 'no-getMemory.wast' doesn't use memory so Asmble creates class file without 'mem' field
        val sumFile = getClass.getResource("/wast/no-getMemory.wast").getPath

        val res = for {
          vm <- WasmVm[IO](Seq(sumFile), cryptoHasher = testHasher)
          state ← vm.getVmState[IO].toVmError
        } yield {
          state.toArray shouldBe testHasher.unsafe(Array.emptyByteArray)
        }

        res.success()
      }

      "there is one module with memory present" in {
        // the code in 'counter.wast' uses 'memory', instance for this module created with 'memory' field
        val counterFile = getClass.getResource("/wast/counter.wast").getPath

        val res = for {
          vm <- WasmVm[IO](Seq(counterFile))
          _ ← vm.invoke[IO](None, "inc", Nil) // 0 -> 1
          get1 ← vm.invoke[IO](None, "get", Nil) // read 1
          state1 ← vm.getVmState[IO]
          get1AfterGettingState ← vm.invoke[IO](None, "get", Nil) // read 1

          _ ← vm.invoke[IO](None, "inc", Nil) // 1 -> 2
          state2 ← vm.getVmState[IO]
          get2AfterGettingState ← vm.invoke[IO](None, "get", Nil).toVmError // read 2
        } yield {
          get1 should not be None
          get1AfterGettingState should not be None
          get2AfterGettingState should not be None

          get1.get.deep shouldBe Array[Byte](1, 0, 0, 0).deep
          get1AfterGettingState.get.deep shouldBe Array[Byte](1, 0, 0, 0).deep
          get2AfterGettingState.get.deep shouldBe Array[Byte](2, 0, 0, 0).deep
          state1.size shouldBe 32
          state2.size shouldBe 32
          state1 should not be state2
        }

        res.success()
      }

      "there are several modules present" in {
        val counterFile = getClass.getResource("/wast/counter.wast").getPath
        val counterCopyFile = getClass.getResource("/wast/counter-copy.wast").getPath
        val mulFile = getClass.getResource("/wast/mul.wast").getPath

        val res = for {
          vm <- WasmVm[IO](Seq(counterCopyFile, mulFile, counterFile))

          _ ← vm.invoke[IO](None, "inc", Nil) // 0 -> 1
          get1 ← vm.invoke[IO](None, "get", Nil) // read 1
          _ ← vm.invoke[IO](Some("CounterCopyModule"), "inc", Nil) // 0 -> 1
          getFromCopy1 ← vm.invoke[IO](Some("CounterCopyModule"), "get", Nil) // read 1
          sum1 ← vm.invoke[IO](Some("MulModule"), "mul", Seq("100", "13")) // read 1

          state1 ← vm.getVmState[IO]

          _ ← vm.invoke[IO](None, "inc", Nil) // 1 -> 2
          _ ← vm.invoke[IO](Some("CounterCopyModule"), "inc", Nil) // 1 -> 2

          state2 ← vm.getVmState[IO].toVmError

        } yield {
          get1 should not be None
          getFromCopy1 should not be None

          get1.get.deep shouldBe Array[Byte](1, 0, 0, 0).deep
          getFromCopy1.get.deep shouldBe Array[Byte](1, 0, 0, 0).deep
          state1.size shouldBe 32
          state2.size shouldBe 32
          state1 should not be state2
        }

        res.success()
      }

      "simple test for string passing" in {
        val simpleStringPassingTestFile = getClass.getResource("/wast/simple-string-passing.wast").getPath

        val res = for {
          vm ← WasmVm[IO](Seq(simpleStringPassingTestFile))
          value1 ← vm.invoke[IO](None, "circular_xor", Seq("\"test_argument\""))
          value2 ← vm.invoke[IO](None, "circular_xor", Seq("\"XX\""))
          value3 ← vm.invoke[IO](None, "circular_xor", Seq("\"XXX\""))
          value4 ← vm.invoke[IO](None, "circular_xor", Seq("\"\"")) // empty string
          value5 ← vm.invoke[IO](None, "circular_xor", Seq("\"\"\"")) // " string
          state ← vm.getVmState[IO].toVmError
        } yield {
          value1 should not be None
          value2 should not be None
          value3 should not be None
          value4 should not be None
          value5 should not be None

          value1.get.deep shouldBe Array[Int](90, 0, 0, 0).deep
          value2.get.deep shouldBe Array[Int](0, 0, 0, 0).deep
          value3.get.deep shouldBe Array[Int]('X'.toInt, 0, 0, 0).deep
          value4.get.deep shouldBe Array[Int](0, 0, 0, 0).deep // this Wasm example returns 0 on empty string
          value5.get.deep shouldBe Array[Int]('"'.toInt, 0, 0, 0).deep
        }

        res.success()
      }

      "string passing" should {
        "raise an error" when {
          "trying to pass incorrect string" in {
            val simpleStringPassingTestFile = getClass.getResource("/wast/simple-string-passing.wast").getPath

            val res = for {
              vm <- WasmVm[IO](Seq(simpleStringPassingTestFile))
              result <- vm.invoke[IO](None, "circular_xor", Seq("\""))
              state ← vm.getVmState[IO].toVmError
            } yield state

            val error = res.failed()
            error.getMessage shouldBe
              "Invalid number of arguments, expected=2, actually=1 for fn='<no-name>.circular_xor' " +
                "(or passed string argument is incorrect or isn't matched the corresponding argument in Wasm function)"
          }

          "trying to use Wasm memory when getMemory function isn't defined" in {
            val noGetMemoryTestFile = getClass.getResource("/wast/no-getMemory.wast").getPath

            val res = for {
              vm <- WasmVm[IO](Seq(noGetMemoryTestFile))
              result <- vm.invoke[IO](None, "test", Seq("\"test\""))
              state ← vm.getVmState[IO].toVmError
            } yield state

            val error = res.failed()
            error.getMessage shouldBe
              "Trying to use absent Wasm memory while injecting string=test"
            error shouldBe a[VmMemoryError]
          }

          "Wasm allocate function returns an incorrect value" in {
            val badAllocationFunctionFile = getClass.getResource("/wast/bad-allocation-function.wast").getPath

            val res = for {
              vm <- WasmVm[IO](Seq(badAllocationFunctionFile))
              result <- vm.invoke[IO](None, "test", Seq("\"test\""))
              state ← vm.getVmState[IO].toVmError
            } yield state

            val error = res.failed()
            error.getMessage shouldBe
              "The Wasm allocation function returned incorrect offset=9223372036854775807"
            error shouldBe a[VmMemoryError]
          }

        }
      }

      "simple test for string returning" in {
        val simpleStringPassingTestFile = getClass.getResource("/wast/simple-string-returning.wast").getPath

        val res = for {
          vm ← WasmVm[IO](Seq(simpleStringPassingTestFile))
          value1 ← vm.invoke[IO](None, "hello", Nil)
          state ← vm.getVmState[IO].toVmError
        } yield {
          value1 should not be None

          val stringValue = new String(value1.get)
          stringValue shouldBe "Hello from Fluence Labs!"
        }

        res.success()
      }

    }
  }

}
