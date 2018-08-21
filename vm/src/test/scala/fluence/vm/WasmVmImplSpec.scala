/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
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

class WasmVmImplSpec extends WordSpec with Matchers {

  "invoke" should {
    "raise error" when {

      "unable to find a function" in {
        val sumFile = getClass.getResource("/wast/sum.wast").getPath
        val res = for {
          vm <- WasmVm[IO](Seq(sumFile))
          result ← vm.invoke[IO](None, "wrongFnName", Seq("100", "13", "extraArg"))
        } yield result
        val error = res.failed()
        error.errorKind shouldBe NoSuchFnError
        error.message should startWith("Unable to find a function with the name='<no-name>.wrongFnName'")
      }

      "invalid number of arguments" in {
        val sumFile = getClass.getResource("/wast/sum.wast").getPath
        val res = for {
          vm <- WasmVm[IO](Seq(sumFile))
          result ← vm.invoke[IO](None, "sum", Seq("100", "13", "extraArg"))
        } yield result
        val error = res.failed()
        error.errorKind shouldBe InvalidArgError
        error.message should startWith(
          "Invalid number of arguments, expected=2, actually=3 for fn='<no-name>.sum'"
        )
      }

      "invalid type for arguments" in {
        val sumFile = getClass.getResource("/wast/sum.wast").getPath
        val res = for {
          vm <- WasmVm[IO](Seq(sumFile))
          result ← vm.invoke[IO](None, "sum", Seq("stringParam", "[1, 2, 3]"))
        } yield result
        val error = res.failed()
        error.errorKind shouldBe InvalidArgError
        error.message should startWith("Arg 0 of 'stringParam' not an int")
      }

      "wasm code fell into the trap" in {
        val sumFile = getClass.getResource("/wast/sum-with-trap.wast").getPath
        val res = for {
          vm <- WasmVm[IO](Seq(sumFile))
          result ← vm.invoke[IO](None, "sum", Seq("100", "13")) // Integer overflow
        } yield result
        val error = res.failed()
        error.errorKind shouldBe TrapError
        error.message should startWith("Function '<no-name>.sum' with args: List(100, 13) was failed")
      }

    }
  }

  "invokes function success" when {
    "run sum.wast" in {
      val sumFile = getClass.getResource("/wast/sum.wast").getPath

      val res = for {
        vm <- WasmVm[IO](Seq(sumFile))
        result ← vm.invoke[IO](None, "sum", Seq("100", "13"))
      } yield result shouldBe Some(113)

      res.success()
    }

    "run sum.wast and after that mul.wast" in {
      val sumFile = getClass.getResource("/wast/sum.wast").getPath
      val mulFile = getClass.getResource("/wast/mul.wast").getPath

      val res = for {
        vm <- WasmVm[IO](Seq(mulFile, sumFile))
        mulResult ← vm.invoke[IO](Some("multiplier"), "mul", Seq("100", "13"))
        sumResult ← vm.invoke[IO](None, "sum", Seq("100", "13"))
      } yield {
        mulResult shouldBe Some(1300)
        sumResult shouldBe Some(113)
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
        get3 ← vm.invoke[IO](None, "get", Nil) //read 3
      } yield {
        get0 shouldBe Some(0)
        get1 shouldBe Some(1)
        get3 shouldBe Some(3)
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
          state ← vm.getVmState[IO]
        } yield state

        val error = res.failed()
        error.message shouldBe "Getting internal state for module=<no-name> failed"
        error.causedBy.get shouldBe a[CryptoError]
        error.errorKind shouldBe InternalVmError
      }

    }

    "returns state" when {
      "there is one module without memory present" in {
        val testHasher = DumbCrypto.testHasher
        // the code in 'sum.wast' don't use 'memory', instance for this module created without 'memory' field
        val sumFile = getClass.getResource("/wast/sum.wast").getPath

        val res = for {
          vm <- WasmVm[IO](Seq(sumFile), cryptoHasher = testHasher)
          result ← vm.invoke[IO](None, "sum", Seq("100", "13"))
          state ← vm.getVmState[IO]
        } yield {
          result shouldBe Some(113)
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
          get2AfterGettingState ← vm.invoke[IO](None, "get", Nil) // read 2
        } yield {
          get1 shouldBe Some(1)
          get1AfterGettingState shouldBe Some(1)
          get2AfterGettingState shouldBe Some(2)
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
          _ ← vm.invoke[IO](Some("counter-copy"), "inc", Nil) // 0 -> 1
          getFromCopy1 ← vm.invoke[IO](Some("counter-copy"), "get", Nil) // read 1
          sum1 ← vm.invoke[IO](Some("multiplier"), "mul", Seq("100", "13")) // read 1

          state1 ← vm.getVmState[IO]

          _ ← vm.invoke[IO](None, "inc", Nil) // 1 -> 2
          _ ← vm.invoke[IO](Some("counter-copy"), "inc", Nil) // 1 -> 2

          state2 ← vm.getVmState[IO]

        } yield {
          get1 shouldBe Some(1)
          getFromCopy1 shouldBe Some(1)
          state1.size shouldBe 32
          state2.size shouldBe 32
          state1 should not be state2
        }

        res.success()
      }

    }
  }

}
