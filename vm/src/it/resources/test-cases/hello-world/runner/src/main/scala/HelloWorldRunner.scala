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

import cats.{Foldable, Traverse}

import scala.concurrent.duration._
import cats.syntax.foldable._
import cats.syntax.list._
import cats.instances.list._
import cats.data.{EitherT, NonEmptyList}
import cats.effect.{ExitCode, IO, IOApp}
import fluence.vm.VmError.InternalVmError
import fluence.vm.{VmError, WasmVm}

import scala.language.higherKinds

/**
 * A hello-world2 examples runner that is an example of possible debugger of `hello_world2` backend applications
 * (written both on Rust 2015 and 2018).
 * Internally it creates WasmVm and invokes the application with some parameters. Also can be used as a template for
 * debugging other backend applications.
 */
object HelloWorldRunner extends IOApp {

  override def run(args: List[String]): IO[ExitCode] = {

//    val inputFile = "/home/diemust/git/fun/dice-game/backend-as/build/optimized.wasm"
    /*val inputFiles = "/home/diemust/git/fun/llamadb/backend-submodule/target/wasm32-unknown-unknown/release/llama_db.wasm" ::
      "/home/diemust/git/fun/signature/target/wasm32-unknown-unknown/release/signature.wasm" ::
      "/home/diemust/git/fun/dice-game/backend-db-as/build/optimized.wasm" :: Nil*/
    val inputFiles = "/home/diemust/.fluence/app-1-0/vmcode/vmcode/bfe4950eb8318d41bdd757d6b004f0ebc62b37203c96f0d85aad4bd71523e8cc.wasm" ::
      "/home/diemust/.fluence/app-1-0/vmcode/vmcode/2370146677e959abbdbf1f05a39e166d157d2541ec3d829e1143c38ce6990518.wasm" ::
      "/home/diemust/.fluence/app-1-0/vmcode/vmcode/a7cd91b3ff4f08ffe65f842a610f7d66c175cdf3046b0470eec59592dd32e3ea.wasm" :: Nil
    val loggerFile = ""
//    val inputFile = "/home/diemust/git/fun/dice-game/backend/target/wasm32-unknown-unknown/release/dice_game.wasm"
//    val inputFile = "/home/diemust/git/example/build/optimized.wasm"

    val program: EitherT[IO, VmError, String] = for {
//      inputFile <- EitherT(getWasmFilePath(args).attempt)
//        .leftMap(e => InternalVmError(e.getMessage, Some(e)))
      vm ← WasmVm[IO](
        NonEmptyList.fromList(inputFiles).get,
        "fluence.vm.debugger"
      )
      initState ← vm.getVmState[IO]

      join = "{\"action\": \"Join\"}"
      result1 ← vm.invoke[IO](None, join.getBytes())
      /*_ ← vm.invoke[IO](None, join.getBytes())
      _ ← vm.invoke[IO](None, join.getBytes())
      _ ← vm.invoke[IO](None, join.getBytes())*/
      _ = println(result1)
      roll = """
               |{
               | "action": "Roll",
               | "player_id": 0,
               | "bet_placement": 2,
               | "bet_size": 1
               |}
               |
        """.stripMargin
      randomString = "3ffcf636c9c862a952d08472832f1425b17b8a4847ef864d93edce81bd5db1e55b5140257d466a400f62008b895e27807ef22fd32dc48099f258d11931f6fee6\n0\nSELECT MAX(age) FROM users"
      getBalance = """
                     |{
                     | "action": "GetBalance",
                     | "player_id": 0
                     |}
                     |
        """.stripMargin
      result2 ← vm.invoke[IO](None, roll.getBytes())
      _ <- vm.invoke[IO](None, join.getBytes())
      _ <- {
        val a: IO[IO[Unit]] = Foldable[List].foldM(List.range(0, 20000), IO.unit) {
          case (acc, v) =>
            for {
//              _ <- IO.sleep(200.millis)
              _ <- acc
              _ <- {
                if (v % 100 == 0) {
                  vm.invoke[IO](None, roll.getBytes()).value.attempt
                } else {
                  IO.unit
                }
              }
              _ <- {
                if (v % 2000 == 0) {
                  vm.getVmState[IO].toVmError.value.attempt
                } else {
                  IO.unit
                }
              }
              res <- vm.invoke[IO](None, roll.getBytes()).value.attempt
              _ = {}
            } yield {
              if (res.isRight && res.right.get.isRight) {
                IO.delay(println(v + " " + new String(res.right.get.right.get)))
              } else {
                IO.unit
              }
            }

        }

        val b: EitherT[IO, VmError, IO[Unit]] = EitherT.liftF(a)
        b.map(_.attempt)
      }

      _ = println(result2)
      result3 ← vm.invoke[IO](None, getBalance.getBytes())
      _ ← vm.invoke[IO](None, getBalance.getBytes())
      _ = println(result3)
      finishState <- vm.getVmState[IO].toVmError
      _ = println("result3")
    } yield {

      /*
        In correct execution the console output should be like this:

        INFO  [hello_world2_2018] John has been successfully greeted
        INFO  [hello_world2_2018]  has been successfully greeted
        INFO  [hello_world2_2018] Peter has been successfully greeted
        [SUCCESS] Execution Results.
        initState=ByteVector(32 bytes, 0x6f3ebb11cb3266aaf12c1eea3f6892c78f563dccb43d6c4b4b153ea10be3decf)
        result1=Hello from Fluence to John
        result2=Hello from Fluence to
        result3=Hello from Fluence to Peter
        finishState=ByteVector(32 bytes, 0x21985fc581505837bc44c21aef9d0ca2538f43064a4475218e374ce8e4ecc329)
       */

      s"[SUCCESS] Execution Results.\n" +
        s"initState=$initState \n" +
        s"initState size=$initState \n" +
        s"result1=${new String(result1)} \n" +
        s"result2=${new String(result2)} \n" +
        s"result3=${new String(result3)} \n" +
        s"finishState=$finishState"
    }

    program.value.map {
      case Left(err) ⇒
        println(s"[Error]: $err cause=${err.getCause}")
        err.printStackTrace()
        ExitCode.Error
      case Right(value) ⇒
        println(value)
        ExitCode.Success
    }
  }

  private def getWasmFilePath(args: List[String]): IO[String] = IO {
    args.headOption match {
      case Some(value) ⇒
        println(s"Starts for input file $value")
        value
      case None ⇒
        throw new IllegalArgumentException("Please provide a full path for wasm file as the first CLI argument!")
    }
  }

}
