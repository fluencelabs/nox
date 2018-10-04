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

package fluence.vm.examples

import cats.data.EitherT
import cats.effect.{ExitCode, IO, IOApp}
import fluence.vm.VmError.InternalVmError
import fluence.vm.WasmVm

import scala.language.higherKinds

// todo will be finished when String will be supported as function params and return type
object LlamaDbRunner extends IOApp {

  override def run(args: List[String]): IO[ExitCode] = {

    val program = for {
      inputFile ← EitherT(getInputFile(args).attempt)
        .leftMap(e ⇒ InternalVmError(e.getMessage, Some(e)))
      vm ← WasmVm[IO](Seq(inputFile))
      initState ← vm.getVmState[IO]

      // select * from Quotations;
      ptr ← vm.invoke[IO](None, "allocate", Seq("100"))
      res ← vm.invoke[IO](None, "do_query", List(ptr.get.toString, "100")) // invoke WASM select
      allocateState ← vm.getVmState[IO]
      _ ← vm.invoke[IO](None, "deallocate", Seq(ptr.get.toString, "100"))
      deallocateState ← vm.getVmState[IO]

      finishState ← vm.getVmState[IO].toVmError
    } yield {
      s"select * from Table; $res \n" +
        s"pointer=$ptr \n" +
        s"stateAfterAlloc=$allocateState,  deallocateState=$deallocateState \n" +
        s"[SUCCESS] Execution Results.\n" +
        s"initState=$initState \n" +
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
        throw new IllegalArgumentException("Full path for counter.wasm is required!")
    }
  }

}
