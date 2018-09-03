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

package fluence.vm.examples
import cats.data.EitherT
import cats.effect.{ExitCode, IO, IOApp}
import fluence.vm.VmError.InternalVmError
import fluence.vm.{VmError, WasmVm}

object CounterRunner extends IOApp {

  override def run(args: List[String]): IO[ExitCode] = {

    val program = for {
      inputFile ← EitherT(getInputFile(args).attempt)
        .leftMap(e ⇒ VmError(e, InternalVmError))
      vm ← WasmVm[IO](Seq(inputFile))
      initState ← vm.getVmState[IO]
      _ ← vm.invoke[IO](None, "inc")
      get1 ← vm.invoke[IO](None, "get")
      _ ← vm.invoke[IO](None, "inc")
      _ ← vm.invoke[IO](None, "inc")
      get2 ← vm.invoke[IO](None, "get")
      finishState ← vm.getVmState[IO]
    } yield {
      s"""
          
        [SUCCESS] Execution Results.

        initState=$initState
        get1=$get1
        get2=$get2
        finishState=$finishState

      """
    }

    program.value.map {
      case Left(err) ⇒
        println(s"[Error]: $err")
        ExitCode.Error
      case Right(value) ⇒
        println(value)
        ExitCode.Success
    }
  }

  private def getInputFile(args: List[String]): IO[String] = IO {
    args.headOption match {
      case Some(value) ⇒
        println(s"Starts for input file $value")
        value
      case None ⇒
        throw new IllegalArgumentException("Full path for counter.wasm is required!")
    }
  }

}
