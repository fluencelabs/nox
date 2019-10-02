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

// TODO: Adapt tests for Wasmer

package fluence.vm

import java.nio.{ByteBuffer, ByteOrder}

import cats.data.NonEmptyList
import cats.effect.{IO, Timer}
import fluence.log.{Log, LogFactory}
import fluence.vm.TestUtils._
import fluence.vm.error.{InitializationError, InvocationError}
import org.scalatest.{Assertion, Matchers, WordSpec}

import scala.concurrent.ExecutionContext
import scala.language.{higherKinds, implicitConversions}

class WasmerWasmVmSpec extends WordSpec with Matchers {

  private implicit val timer: Timer[IO] = IO.timer(ExecutionContext.global)
  private implicit val log: Log[IO] = LogFactory.forPrintln[IO](Log.Error).init(getClass.getSimpleName).unsafeRunSync()

  /**
   * By element comparision of arrays.
   */
  private def compareArrays(first: Array[Byte], second: Array[Byte]): Assertion =
    first.deep shouldBe second.deep

  /**
   * Converts ints to byte array by supplied byte order.
   *
   * @param ints array of int
   * @param byteOrder byte order that used for int converting
   */
  private def intsToBytes(
    ints: List[Int],
    byteOrder: ByteOrder = ByteOrder.LITTLE_ENDIAN
  ): ByteBuffer = {
    val intBytesSize = 4
    val converter = ByteBuffer.allocate(intBytesSize * ints.length)

    converter.order(byteOrder)
    ints.foreach(converter.putInt)
    converter.flip()
    converter
  }

  "invoke" should {
    "raise error" when {

      "trying to invoke when a module doesn't have one" ignore {
        val noInvokeTestFile = getClass.getResource("/wast/no-invoke.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(noInvokeTestFile))
          result ← vm.invoke[IO]().toVmError
        } yield result
        val error = res.failed()
        error shouldBe a[InitializationError]
        error.getMessage should startWith("The main module must have functions with names")
      }

      "trying to use Wasm memory when getMemory function isn't defined" ignore {
        val noGetMemoryTestFile = getClass.getResource("/wast/no-getMemory.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(noGetMemoryTestFile))
          _ ← vm.invoke[IO]("test".getBytes())
          state ← vm.computeVmState[IO].toVmError
        } yield state

        val error = res.failed()
        error.getMessage should
          startWith("Unable to initialize module=null")
        error shouldBe a[InitializationError]
      }

      "wasm code falls into the trap" ignore {
        val sumTestFile = getClass.getResource("/wast/sum-with-trap.wast").getPath
        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(sumTestFile))
          result ← vm.invoke[IO](fnArgument = intsToBytes(100 :: 13 :: Nil).array()).toVmError // Integer overflow
        } yield result
        val error = res.failed()
        error shouldBe a[InvocationError]
        error.getMessage should startWith("Function invoke with args:")
        error.getMessage should include("was failed")
      }

      "Wasm allocate function returns an incorrect i64 value" ignore {
        val badAllocationFunctionFile = getClass.getResource("/wast/bad-allocation-function-i64.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(badAllocationFunctionFile))
          _ ← vm.invoke[IO]("test".getBytes())
          state ← vm.computeVmState[IO].toVmError
        } yield state

        val error = res.failed()
        error.getMessage shouldBe "Writing to -1 failed"
        error shouldBe a[InvocationError]
      }

      "Wasm allocate function returns an incorrect f64 value" ignore {
        val badAllocationFunctionFile = getClass.getResource("/wast/bad-allocation-function-f64.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(badAllocationFunctionFile))
          result ← vm.invoke[IO]("test".getBytes())
          state ← vm.computeVmState[IO].toVmError
        } yield state

        val error = res.failed()
        error.getMessage shouldBe "Writing to 200000000 failed"
        error shouldBe a[InvocationError]
      }

      "trying to extract array with incorrect size from Wasm memory" ignore {
        val incorrectArrayReturningTestFile = getClass.getResource("/wast/incorrect-array-returning.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(incorrectArrayReturningTestFile))
          result ← vm.invoke[IO]().toVmError
        } yield result

        val error = res.failed()
        error shouldBe a[InvocationError]
        error.getMessage shouldBe "Reading from offset=1048596 16777215 bytes failed"
      }

    }
  }

  "invokes function success" when {
    "run sum.wast" ignore {
      val sumTestFile = getClass.getResource("/wast/sum.wast").getPath

      val res = for {
        vm ← WasmVm[IO](NonEmptyList.one(sumTestFile))
        result ← vm.invoke[IO](intsToBytes(100 :: 17 :: Nil).array()).toVmError
      } yield {
        compareArrays(result.output, Array[Byte](117, 0, 0, 0))
      }

      res.success()
    }

    "run counter.wast" ignore {
      val counterTestFile = getClass.getResource("/wast/counter.wast").getPath

      val res = for {
        vm ← WasmVm[IO](NonEmptyList.one(counterTestFile))
        get1 ← vm.invoke[IO]() // 0 -> 1; read 1
        get2 ← vm.invoke[IO]() // 1 -> 2; read 2
        get3 ← vm.invoke[IO]().toVmError // 2 -> 3; read 3
      } yield {
        compareArrays(get1.output, Array[Byte](1, 0, 0, 0))
        compareArrays(get2.output, Array[Byte](2, 0, 0, 0))
        compareArrays(get3.output, Array[Byte](3, 0, 0, 0))
      }

      res.success()
    }

    "run simple test with array passsing" ignore {
      val simpleStringPassingTestFile = getClass.getResource("/wast/simple-string-passing.wast").getPath

      val res = for {
        vm ← WasmVm[IO](NonEmptyList.one(simpleStringPassingTestFile))
        value1 ← vm.invoke[IO]("test_argument".getBytes())
        value2 ← vm.invoke[IO]("XX".getBytes())
        value3 ← vm.invoke[IO]("XXX".getBytes())
        value4 ← vm.invoke[IO]("".getBytes()) // empty string
        value5 ← vm.invoke[IO]("\"".getBytes()).toVmError // " string
      } yield {
        compareArrays(value1.output, Array[Byte](90, 0, 0, 0))
        compareArrays(value2.output, Array[Byte](0, 0, 0, 0))
        compareArrays(value3.output, Array[Byte]('X'.toByte, 0, 0, 0))
        compareArrays(value4.output, Array[Byte](0, 0, 0, 0)) // this Wasm example returns 0 on empty strings
        compareArrays(value5.output, Array[Byte]('"'.toByte, 0, 0, 0))
      }

      res.success()
    }

    "run simple test with array returning" ignore {
      val simpleArrayPassingTestFile = getClass.getResource("/wast/simple-array-returning.wast").getPath

      val res = for {
        vm ← WasmVm[IO](NonEmptyList.one(simpleArrayPassingTestFile))
        value1 ← vm.invoke[IO]()
        _ ← vm.computeVmState[IO].toVmError
      } yield {
        val stringValue = new String(value1.output)
        stringValue shouldBe "Hello from Fluence Labs!"
      }

      res.success()
    }

    "run simple test with array mutation" ignore {
      val simpleArrayMutationTestFile = getClass.getResource("/wast/simple-array-mutation.wast").getPath

      val res = for {
        vm ← WasmVm[IO](NonEmptyList.one(simpleArrayMutationTestFile))
        value1 ← vm.invoke[IO]("AAAAAAA".getBytes())
        state ← vm.computeVmState[IO].toVmError
      } yield {
        val stringValue = new String(value1.output)
        stringValue shouldBe "BBBBBBB"
      }

      res.success()
    }

  }

  "getVmState" should {
    "returns state" when {
      "there is one module with memory present" ignore {
        // the code in 'counter.wast' uses 'memory', instance for this module created with 'memory' field
        val counterTestFile = getClass.getResource("/wast/counter.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(counterTestFile))
          get1 ← vm.invoke[IO]() // 0 -> 1; return 1
          state1 ← vm.computeVmState[IO]
          get2 ← vm.invoke[IO]() // 1 -> 2; return 2

          get3 ← vm.invoke[IO]() // 2 -> 3; return 3
          state2 ← vm.computeVmState[IO]
          get4 ← vm.invoke[IO]().toVmError // 3 -> 4; return 4
        } yield {
          compareArrays(get1.output, Array[Byte](1, 0, 0, 0))
          compareArrays(get2.output, Array[Byte](2, 0, 0, 0))
          compareArrays(get3.output, Array[Byte](3, 0, 0, 0))
          compareArrays(get4.output, Array[Byte](4, 0, 0, 0))

          state1.size shouldBe 32
          state2.size shouldBe 32
          state1 should not be state2
        }

        res.success()
      }

      "there are several modules present" ignore {
        val counterTestFile = getClass.getResource("/wast/counter.wast").getPath
        val counterCopyTestFile = getClass.getResource("/wast/counter-copy.wast").getPath
        val mulTestFile = getClass.getResource("/wast/mul.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.of(counterTestFile, counterCopyTestFile, mulTestFile))

          get1 ← vm.invoke[IO]() // 0 -> 1; read 1
          _ ← vm.invoke[IO]() // 1 -> 2; read 2

          state1 ← vm.computeVmState[IO]

          _ ← vm.invoke[IO]() // 2 -> 3
          get2 ← vm.invoke[IO]() // 3 -> 4

          state2 ← vm.computeVmState[IO].toVmError

        } yield {
          compareArrays(get1.output, Array[Byte](1, 0, 0, 0))
          compareArrays(get2.output, Array[Byte](4, 0, 0, 0))

          state1.size shouldBe 32
          state2.size shouldBe 32
          state1 should not be state2
        }

        res.success()
      }

    }
  }

}
