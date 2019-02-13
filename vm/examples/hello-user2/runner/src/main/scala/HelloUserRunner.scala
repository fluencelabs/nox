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

package fluence.vm.runner

import cats.data.{EitherT, NonEmptyList}
import cats.effect.{ExitCode, IO, IOApp}
import fluence.vm.VmError.InternalVmError
import fluence.vm.{VmError, WasmVm}

import scala.language.higherKinds

object HelloUserRunner extends IOApp {

  override def run(args: List[String]): IO[ExitCode] = {

    val program: EitherT[IO, VmError, String] = for {
      inputFile <- EitherT(getInputFile(args).attempt)
        .leftMap(e => InternalVmError(e.getMessage, Some(e)))
      vm ← WasmVm[IO](NonEmptyList.one(inputFile))
      initState ← vm.getVmState[IO]

      result1 ← vm.invoke(None, "John".getBytes())
      result2 ← vm.invoke(None, "".getBytes())
      result3 ← vm.invoke(None, "Peter".getBytes())

      finishState <- vm.getVmState[IO].toVmError
    } yield {
      s"[SUCCESS] Execution Results.\n" +
        s"initState=$initState \n" +
        s"result1=$result1 \n" +
        s"result2=$result2 \n" +
        s"result3=$result3 \n" +
        s"finishState=$finishState"
    }

    program.value.map {
      case Left(err) ⇒
        println(s"[Error]: $err cause=${err.getCause}")
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
        throw new IllegalArgumentException("Full path for wasm file is required!")
    }
  }

}