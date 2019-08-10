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

import java.nio.{ByteBuffer, ByteOrder}

import asmble.compile.jvm.MemoryBuffer
import cats.data.{EitherT, NonEmptyList}
import cats.effect.{IO, Timer}
import fluence.crypto.CryptoError
import fluence.log.{Log, LogFactory}
import fluence.vm.TestUtils._
import fluence.vm.VmError._
import fluence.vm.wasm.MemoryHasher
import org.scalatest.{Assertion, Matchers, WordSpec}

import scala.concurrent.ExecutionContext
import scala.language.{higherKinds, implicitConversions}

class AsmbleWasmVmSpec extends WordSpec with Matchers {

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

      "trying to invoke when a module doesn't have one" in {
        val noInvokeTestFile = getClass.getResource("/wast/no-invoke.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(noInvokeTestFile), MemoryHasher[IO])
          result ← vm.invoke[IO]().toVmError
        } yield result
        val error = res.failed()
        error shouldBe a[NoSuchFnError]
        error.getMessage should startWith("The main module must have functions with names")
      }

      "trying to use Wasm memory when getMemory function isn't defined" in {
        val noGetMemoryTestFile = getClass.getResource("/wast/no-getMemory.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(noGetMemoryTestFile), MemoryHasher[IO])
          _ ← vm.invoke[IO]("test".getBytes())
          state ← vm.getVmState[IO].toVmError
        } yield state

        val error = res.failed()
        error.getMessage should
          startWith("Unable to initialize module=null")
        error shouldBe a[InitializationError]
      }

      "wasm code falls into the trap" in {
        val sumTestFile = getClass.getResource("/wast/sum-with-trap.wast").getPath
        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(sumTestFile), MemoryHasher[IO])
          result ← vm.invoke[IO](fnArgument = intsToBytes(100 :: 13 :: Nil).array()).toVmError // Integer overflow
        } yield result
        val error = res.failed()
        error shouldBe a[TrapError]
        error.getMessage should startWith("Function invoke with args:")
        error.getMessage should include("was failed")
      }

      "Wasm allocate function returns an incorrect i64 value" in {
        val badAllocationFunctionFile = getClass.getResource("/wast/bad-allocation-function-i64.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(badAllocationFunctionFile), MemoryHasher[IO])
          _ ← vm.invoke[IO]("test".getBytes())
          state ← vm.getVmState[IO].toVmError
        } yield state

        val error = res.failed()
        error.getMessage shouldBe "Writing to -1 failed"
        error shouldBe a[VmMemoryError]
      }

      "Wasm allocate function returns an incorrect f64 value" in {
        val badAllocationFunctionFile = getClass.getResource("/wast/bad-allocation-function-f64.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(badAllocationFunctionFile), MemoryHasher[IO])
          result ← vm.invoke[IO]("test".getBytes())
          state ← vm.getVmState[IO].toVmError
        } yield state

        val error = res.failed()
        error.getMessage shouldBe "Writing to 200000000 failed"
        error shouldBe a[VmMemoryError]
      }

      "trying to extract array with incorrect size from Wasm memory" in {
        val incorrectArrayReturningTestFile = getClass.getResource("/wast/incorrect-array-returning.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(incorrectArrayReturningTestFile), MemoryHasher[IO])
          result ← vm.invoke[IO]().toVmError
        } yield result

        val error = res.failed()
        error shouldBe a[VmMemoryError]
        error.getMessage shouldBe "Reading from offset=1048596 16777215 bytes failed"
      }

    }
  }

  "invokes function success" when {
    "run sum.wast" in {
      val sumTestFile = getClass.getResource("/wast/sum.wast").getPath

      val res = for {
        vm ← WasmVm[IO](NonEmptyList.one(sumTestFile), MemoryHasher[IO])
        result ← vm.invoke[IO](intsToBytes(100 :: 17 :: Nil).array()).toVmError
      } yield {
        compareArrays(result.output, Array[Byte](117, 0, 0, 0))
      }

      res.success()
    }

    "run counter.wast" in {
      val counterTestFile = getClass.getResource("/wast/counter.wast").getPath

      val res = for {
        vm ← WasmVm[IO](NonEmptyList.one(counterTestFile), MemoryHasher[IO])
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

    "run simple test with array passsing" in {
      val simpleStringPassingTestFile = getClass.getResource("/wast/simple-string-passing.wast").getPath

      val res = for {
        vm ← WasmVm[IO](NonEmptyList.one(simpleStringPassingTestFile), MemoryHasher[IO])
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

    "run simple test with array returning" in {
      val simpleArrayPassingTestFile = getClass.getResource("/wast/simple-array-returning.wast").getPath

      val res = for {
        vm ← WasmVm[IO](NonEmptyList.one(simpleArrayPassingTestFile), MemoryHasher[IO])
        value1 ← vm.invoke[IO]()
        _ ← vm.getVmState[IO].toVmError
      } yield {
        val stringValue = new String(value1.output)
        stringValue shouldBe "Hello from Fluence Labs!"
      }

      res.success()
    }

    "run simple test with array mutation" in {
      val simpleArrayMutationTestFile = getClass.getResource("/wast/simple-array-mutation.wast").getPath

      val res = for {
        vm ← WasmVm[IO](NonEmptyList.one(simpleArrayMutationTestFile), MemoryHasher[IO])
        value1 ← vm.invoke[IO]("AAAAAAA".getBytes())
        state ← vm.getVmState[IO].toVmError
      } yield {
        val stringValue = new String(value1.output)
        stringValue shouldBe "BBBBBBB"
      }

      res.success()
    }

  }

  "getVmState" should {
    "raise an error" when {
      "getting inner state for module failed" in {
        val counterTestFile = getClass.getResource("/wast/counter.wast").getPath
        val badHasher: MemoryHasher.Builder[IO] =
          (v1: MemoryBuffer) => EitherT.leftT(VmMemoryError("error", Some(CryptoError("Error"))))

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(counterTestFile), badHasher)
          state ← vm.getVmState[IO].toVmError
        } yield state

        val error = res.failed()
        error.getMessage shouldBe "Unable to instantiate WasmModuleMemory for module=null"
        // TODO error.getCause shouldBe a[CryptoError]
        // TODO error shouldBe a[InternalVmError]
      }

    }

    "returns state" when {
      "there is one module with memory present" in {
        // the code in 'counter.wast' uses 'memory', instance for this module created with 'memory' field
        val counterTestFile = getClass.getResource("/wast/counter.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.one(counterTestFile), MemoryHasher[IO])
          get1 ← vm.invoke[IO]() // 0 -> 1; return 1
          state1 ← vm.getVmState[IO]
          get2 ← vm.invoke[IO]() // 1 -> 2; return 2

          get3 ← vm.invoke[IO]() // 2 -> 3; return 3
          state2 ← vm.getVmState[IO]
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

      "there are several modules present" in {
        val counterTestFile = getClass.getResource("/wast/counter.wast").getPath
        val counterCopyTestFile = getClass.getResource("/wast/counter-copy.wast").getPath
        val mulTestFile = getClass.getResource("/wast/mul.wast").getPath

        val res = for {
          vm ← WasmVm[IO](NonEmptyList.of(counterTestFile, counterCopyTestFile, mulTestFile), MemoryHasher[IO])

          get1 ← vm.invoke[IO]() // 0 -> 1; read 1
          _ ← vm.invoke[IO]() // 1 -> 2; read 2

          state1 ← vm.getVmState[IO]

          _ ← vm.invoke[IO]() // 2 -> 3
          get2 ← vm.invoke[IO]() // 3 -> 4

          state2 ← vm.getVmState[IO].toVmError

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
