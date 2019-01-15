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

import cats.Monad
import cats.data.{EitherT, NonEmptyList}
import cats.effect.IO
import fluence.crypto.{Crypto, CryptoError, DumbCrypto}
import fluence.vm.TestUtils._
import fluence.vm.VmError._
import org.scalatest.{Matchers, WordSpec}

import scala.language.{higherKinds, implicitConversions}

class AsmbleWasmVmSpec extends WordSpec with Matchers {

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
          vm <- WasmVm[IO](NonEmptyList.one(noInvokeTestFile))
          result ← vm.invoke[IO]().toVmError
        } yield result
        val error = res.failed()
        error shouldBe a[NoSuchFnError]
        error.getMessage should startWith("Unable to find a function with the name='<no-name>.wrongFnName'")
      }

      "trying to use Wasm memory when getMemory function isn't defined" in {
        val noGetMemoryTestFile = getClass.getResource("/wast/no-getMemory.wast").getPath

        val res = for {
          vm <- WasmVm[IO](NonEmptyList.one(noGetMemoryTestFile))
          result <- vm.invoke[IO](None, "test".getBytes())
          state ← vm.getVmState[IO].toVmError
        } yield state

        val error = res.failed()
        error.getMessage should startWith("Trying to use absent Wasm memory while injecting")
        error shouldBe a[VmMemoryError]
      }

      "wasm code falls into the trap" in {
        val sumTestFile = getClass.getResource("/wast/sum-with-trap.wast").getPath
        val res = for {
          vm <- WasmVm[IO](NonEmptyList.one(sumTestFile))
          result ← vm.invoke[IO](fnArgument = intsToBytes(100 :: 13 :: Nil).array()).toVmError // Integer overflow
        } yield result
        val error = res.failed()
        error shouldBe a[TrapError]
        error.getMessage should startWith("Function '<no-name>.sum' with args:")
        error.getMessage should endWith("was failed")
      }

      "Wasm allocate function returns an incorrect value" in {
        val badAllocationFunctionFile = getClass.getResource("/wast/bad-allocation-function.wast").getPath

        val res = for {
          vm <- WasmVm[IO](NonEmptyList.one(badAllocationFunctionFile))
          result <- vm.invoke[IO](None, "test".getBytes())
          state ← vm.getVmState[IO].toVmError
        } yield state

        val error = res.failed()
        error.getMessage shouldBe
          "The Wasm allocation function returned incorrect offset=9223372036854775807"
        error shouldBe a[VmMemoryError]
      }

      "trying to extract array with incorrect size from Wasm memory" in {
        val simpleArrayPassingTestFile = getClass.getResource("/wast/simple-array-returning.wast").getPath

        val res = for {
          vm <- WasmVm[IO](NonEmptyList.one(simpleArrayPassingTestFile))
          result ← vm.invoke[IO]().toVmError
        } yield result

        val error = res.failed()
        error shouldBe a[VmMemoryError]
        error.getMessage shouldBe "String reading from offset=1048592 failed"
      }

    }
  }

  "invokes function success" when {
    "run sum.wast" in {
      val sumTestFile = getClass.getResource("/wast/sum.wast").getPath

      val res = for {
        vm <- WasmVm[IO](NonEmptyList.one(sumTestFile))
        result ← vm.invoke[IO](Some("SumModule"), intsToBytes(100 :: 13 :: Nil).array()).toVmError
      } yield {
        result should not be None
        result.get.deep shouldBe Array[Byte](113, 0, 0, 0).deep
      }

      res.success()
    }

    "run sum.wast and after that mul.wast" in {
      val sumTestFile = getClass.getResource("/wast/sum.wast").getPath
      val mulTestFile = getClass.getResource("/wast/mul.wast").getPath

      val res = for {
        vm <- WasmVm[IO](NonEmptyList.of(mulTestFile, sumTestFile))
        mulResult ← vm.invoke[IO](Some("MulModule"), intsToBytes(100 :: 13 :: Nil).array())
        sumResult ← vm.invoke[IO](Some("SumModule"), intsToBytes(100 :: 13 :: Nil).array()).toVmError
      } yield {
        mulResult should not be None
        sumResult should not be None

        mulResult.get.deep shouldBe Array[Byte](20, 5, 0, 0).deep
        sumResult.get.deep shouldBe Array[Byte](113, 0, 0, 0).deep
      }

      res.success()
    }

    "run counter.wast" in {
      val counterTestFile = getClass.getResource("/wast/counter.wast").getPath

      val res = for {
        vm <- WasmVm[IO](NonEmptyList.one(counterTestFile))
        get1 ← vm.invoke[IO]()           // 0 -> 1
        get2 ← vm.invoke[IO]()           // 0 -> 1
        get3 ← vm.invoke[IO]().toVmError // 0 -> 1
      } yield {
        get1 should not be None
        get2 should not be None
        get3 should not be None

        get1.get.deep shouldBe Array[Byte](0, 0, 0, 0).deep
        get2.get.deep shouldBe Array[Byte](1, 0, 0, 0).deep
        get3.get.deep shouldBe Array[Byte](3, 0, 0, 0).deep
      }

      res.success()
    }

    "run simple test with array passsing" in {
      val simpleStringPassingTestFile = getClass.getResource("/wast/simple-string-passing.wast").getPath

      val res = for {
        vm ← WasmVm[IO](NonEmptyList.one(simpleStringPassingTestFile))
        value1 ← vm.invoke[IO](None, "test_argument".getBytes())
        value2 ← vm.invoke[IO](None, "XX".getBytes())
        value3 ← vm.invoke[IO](None, "XXX".getBytes())
        value4 ← vm.invoke[IO](None, "".getBytes()) // empty string
        value5 ← vm.invoke[IO](None, "\"".getBytes()).toVmError // " string
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

    "run simple test with array returning" in {
      val simpleArrayPassingTestFile = getClass.getResource("/wast/simple-array-returning.wast").getPath

      val res = for {
        vm ← WasmVm[IO](NonEmptyList.one(simpleArrayPassingTestFile))
        value1 ← vm.invoke[IO]()
        state ← vm.getVmState[IO].toVmError
      } yield {
        value1 should not be None

        val stringValue = new String(value1.get)
        stringValue shouldBe "Hello from Fluence Labs!"
      }

      res.success()
    }

    "run simple test with array mutation" in {
      val simpleArrayMutationTestFile = getClass.getResource("/wast/simple-array-mutation.wast").getPath

      val res = for {
        vm ← WasmVm[IO](NonEmptyList.one(simpleArrayMutationTestFile))
        value1 ← vm.invoke[IO](None, "AAAAAAA".getBytes())
        state ← vm.getVmState[IO].toVmError
      } yield {
        value1 should not be None

        val stringValue = new String(value1.get)
        stringValue shouldBe "BBBBBBB"
      }

      res.success()
    }

  }

  "getVmState" should {
    "raise an error" when {
      "getting inner state for module failed" in {
        val counterTestFile = getClass.getResource("/wast/counter.wast").getPath
        val badHasher = new Crypto.Hasher[Array[Byte], Array[Byte]] {
          override def apply[F[_]: Monad](input: Array[Byte]): EitherT[F, CryptoError, Array[Byte]] =
            EitherT.leftT(CryptoError("error!"))
        }
        val res = for {
          vm <- WasmVm[IO](NonEmptyList.one(counterTestFile), cryptoHasher = badHasher)
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
        val sumTestFile = getClass.getResource("/wast/no-getMemory.wast").getPath

        val res = for {
          vm <- WasmVm[IO](NonEmptyList.one(sumTestFile), cryptoHasher = testHasher)
          state ← vm.getVmState[IO].toVmError
        } yield {
          state.toArray shouldBe testHasher.unsafe(Array.emptyByteArray)
        }

        res.success()
      }

      "there is one module with memory present" in {
        // the code in 'counter.wast' uses 'memory', instance for this module created with 'memory' field
        val counterTestFile = getClass.getResource("/wast/counter.wast").getPath

        val res = for {
          vm <- WasmVm[IO](NonEmptyList.one(counterTestFile))
          get1 ← vm.invoke[IO]() // 0 -> 1; return 1
          state1 ← vm.getVmState[IO]
          get2 ← vm.invoke[IO]() // 1 -> 2; return 2

          get3 ← vm.invoke[IO]() // 2 -> 3; return 3
          state2 ← vm.getVmState[IO]
          get4 ← vm.invoke[IO]().toVmError // 3 -> 4; return 4
        } yield {
          get1 should not be None
          get2 should not be None
          get3 should not be None
          get4 should not be None

          get1.get.deep shouldBe Array[Byte](1, 0, 0, 0).deep
          get2.get.deep shouldBe Array[Byte](2, 0, 0, 0).deep
          get3.get.deep shouldBe Array[Byte](3, 0, 0, 0).deep
          get4.get.deep shouldBe Array[Byte](4, 0, 0, 0).deep
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
          vm <- WasmVm[IO](NonEmptyList.of(counterTestFile, counterCopyTestFile, mulTestFile))

          get1 ← vm.invoke[IO]() // 0 -> 1; read 1
          getFromCopy1 ← vm.invoke[IO](Some("CounterCopyModule")) // read 1
          mul ← vm.invoke[IO](Some("MulModule"), intsToBytes(100 :: 13 :: Nil).array())

          state1 ← vm.getVmState[IO]

          _ ← vm.invoke[IO]() // 1 -> 2
          _ ← vm.invoke[IO](Some("CounterCopyModule")) // 1 -> 2

          state2 ← vm.getVmState[IO].toVmError

        } yield {
          get1 should not be None
          getFromCopy1 should not be None
          mul should not be None

          get1.get.deep shouldBe Array[Byte](1, 0, 0, 0).deep
          getFromCopy1.get.deep shouldBe Array[Byte](1, 0, 0, 0).deep
          mul.get.deep shouldBe Array[Byte](20, 5, 0, 0).deep
          state1.size shouldBe 32
          state2.size shouldBe 32
          state1 should not be state2
        }

        res.success()
      }

    }
  }

}
