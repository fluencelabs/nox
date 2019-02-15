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

import cats.data.{EitherT, NonEmptyList}
import cats.effect.{ExitCode, IO, IOApp}
import fluence.vm.VmError.InternalVmError
import fluence.vm.{VmError, WasmVm}

import scala.language.higherKinds

/**
 * A hello-user2 example runner that is an example of possible debugger of `hello-user2` backend application.
 * Internally it creates WasmVm and invokes the application with some parameters. Also can be used as a template for
 * debugging other backend applications.
 */
object HelloUserRunner extends IOApp {

  override def run(args: List[String]): IO[ExitCode] = {

    val program: EitherT[IO, VmError, String] = for {
      inputFile <- EitherT(getInputFile(args).attempt)
        .leftMap(e => InternalVmError(e.getMessage, Some(e)))
      vm ← WasmVm[IO](NonEmptyList.one(inputFile), "fluence.vm.debugger")
      initState ← vm.getVmState[IO]

      result1 ← vm.invoke[IO](None, "John".getBytes())
      result2 ← vm.invoke[IO](None, "".getBytes())
      result3 ← vm.invoke[IO](None, "Peter".getBytes())

      finishState <- vm.getVmState[IO].toVmError
    } yield {

      /*
        In correct execution the console output should be like this:

        INFO  [hello_user2] John has been successfully greeted
        INFO  [hello_user2]  has been successfully greeted
        INFO  [hello_user2] Peter has been successfully greeted
        [SUCCESS] Execution Results.
        initState=ByteVector(32 bytes, 0x6f3ebb11cb3266aaf12c1eea3f6892c78f563dccb43d6c4b4b153ea10be3decf)
        result1=Hello from Fluence to John
        result2=Hello from Fluence to
        result3=Hello from Fluence to Peter
        finishState=ByteVector(32 bytes, 0x21985fc581505837bc44c21aef9d0ca2538f43064a4475218e374ce8e4ecc329)
       */

      s"[SUCCESS] Execution Results.\n" +
        s"initState=$initState \n" +
        s"result1=${new String(result1)} \n" +
        s"result2=${new String(result2)} \n" +
        s"result3=${new String(result3)} \n" +
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
