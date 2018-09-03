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

import cats.data.EitherT
import cats.effect.IO
import fluence.vm.VmError._
import org.scalatest.{Matchers, WordSpec}

import scala.language.implicitConversions

class WasmVmSpec extends WordSpec with Matchers {

  implicit def error[E](either: EitherT[IO, E, _]): E = either.value.unsafeRunSync().left.get

  "WasmVm" should {

    "raise error" when {

      "config error" in {
        val res = for {
          vm <- WasmVm[IO](Seq("unknown file"), "wrong config namespace")
          result ← vm.invoke[IO](Some("module"), "add20", Seq("100"))
        } yield result

        val error = res.failed()
        error.errorKind shouldBe InternalVmError
        error.message should startWith("Unable to read a config for the namespace")
      }

      "file not found" in {
        val res = for {
          vm <- WasmVm[IO](Seq("unknown file"))
          result ← vm.invoke[IO](Some("module"), "add20", Seq("100"))
        } yield result

        val error = res.failed()
        error.errorKind shouldBe InitializationError
        error.message should startWith("Preparing execution context before execution was failed for")
      }

      "2 module has function with equal name" in {
        val sum1File = getClass.getResource("/wast/sum.wast").getPath // module without name with fn "sum"
        val sum2File = getClass.getResource("/wast/sum-copy.wast").getPath // module without name with fn "sum"

        val res = for {
          vm <- WasmVm[IO](Seq(sum1File, sum2File))
          sumResult ← vm.invoke[IO](None, "sum", Seq("100", "13"))
        } yield sumResult

        val error = res.failed()
        error.errorKind shouldBe InitializationError
        error.message should startWith("Preparing execution context before execution was failed for")
      }

      "unable to find a function" in {
        val sumFile = getClass.getResource("/wast/sum.wast").getPath
        val res = for {
          vm <- WasmVm[IO](Seq(sumFile))
          result ← vm.invoke[IO](None, "wrongFnName", Seq("100", "13", "extraArg"))
        } yield result
        val error = res.failed()
        error.errorKind shouldBe NoSuchFnError
        error.message should startWith("Unable to find a function with the name='<unknown>.wrongFnName'")
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
          "Invalid number of arguments, expected=2, actually=3 for fn='<unknown>.sum'"
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
        error.message should startWith("Function '<unknown>.sum' with args: List(100, 13) was failed")
      }

      // todo add more error cases with prepareContext and module initialization
    }
  }

  "initialize yourself and invoke function success " when {
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
        mulResult ← vm.invoke[IO](Some("$multiplier"), "mul", Seq("100", "13"))
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

    // todo write more success cases ()
  }

  private def invoke(
    inputFile: String,
    action: WasmVm ⇒ EitherT[IO, VmError, Option[Any]],
    expected: Option[Any]
  ): EitherT[IO, VmError, _] =
    for {
      vm <- WasmVm[IO](Seq(inputFile))
      res ← action(vm)
    } yield res shouldBe expected

}
