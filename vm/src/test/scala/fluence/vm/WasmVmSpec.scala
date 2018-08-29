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
import fluence.vm.TestUtils._
import org.scalatest.{Matchers, WordSpec}

import scala.language.implicitConversions

class WasmVmSpec extends WordSpec with Matchers {

  implicit def error[E](either: EitherT[IO, E, _]): E = either.value.unsafeRunSync().left.get

  "apply" should {

    "raise error" when {

      "config error" in {
        val res = for {
          vm <- WasmVm[IO](Seq("unknown file"), "wrong config namespace")
        } yield vm

        val error = res.failed()
        error.errorKind shouldBe InternalVmError
        error.message should startWith("Unable to read a config for the namespace")
      }

      "file not found" in {
        val res = for {
          vm <- WasmVm[IO](Seq("unknown file"))
        } yield vm

        val error = res.failed()
        error.errorKind shouldBe InitializationError
        error.message should startWith("Preparing execution context before execution was failed for")
      }

      "2 module has function with equal name" in {
        val sum1File = getClass.getResource("/wast/sum.wast").getPath // module without name with fn "sum"
        val sum2File = getClass.getResource("/wast/sum-copy.wast").getPath // module without name with fn "sum"

        val res = for {
          vm <- WasmVm[IO](Seq(sum1File, sum2File))
        } yield vm

        val error = res.failed()
        error.errorKind shouldBe InitializationError
        error.message should startWith("The function '<no-name>.sum' was already registered")
      }

      // todo add more error cases with prepareContext and module initialization
    }
  }

  "initialize Vm success" when {
    "there is one file" in {
      val sumFile = getClass.getResource("/wast/sum.wast").getPath

      WasmVm[IO](Seq(sumFile)).success()
    }

    "there are two files" in {
      val sumFile = getClass.getResource("/wast/sum.wast").getPath
      val mulFile = getClass.getResource("/wast/mul.wast").getPath

      WasmVm[IO](Seq(mulFile, sumFile)).success()
    }

  }

}
