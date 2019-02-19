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
import org.json4s._
import org.json4s.jackson.JsonMethods._

import scala.language.higherKinds

/**
 * A tic-tac-toe example runner that is an example of possible debugger of `tic-tac-toe` backend application.
 * Internally it creates WasmVm and invokes the application with some parameters. Also can be used as a template for
 * debugging other backend applications.
 */
object TicTacToeRunner extends IOApp {

  override def run(args: List[String]): IO[ExitCode] = {

    val inputFile = "/Users/trofim/Desktop/work/fluence/fluence/vm/examples/tic-tac-toe/app/target/wasm32-unknown-unknown/release/tic_tac_toe.wasm"
    val program: EitherT[IO, VmError, String] = for {
//      inputFile <- EitherT(getInputFile(args).attempt)
//        .leftMap(e => InternalVmError(e.getMessage, Some(e)))
      vm ← WasmVm[IO](NonEmptyList.one(inputFile), "fluence.vm.debugger")
      initState ← vm.getVmState[IO]

      createPlayer = generateCreatePlayerJson("John", "secret_key")
      createGame = generateCreateGameJson("John", "secret_key", 'X')
      createGame2 = generateCreateGameJson("John2", "secret_key", 'X')
      createPlayer2 = generateCreatePlayerJson("John2", "secret_key")
      makeMove = generateMoveJson("John", "secret_key", 0, 0)

      result1 ← vm.invoke[IO](None, createPlayer.getBytes())
      result2 ← vm.invoke[IO](None, createPlayer2.getBytes())
      result3 ← vm.invoke[IO](None, createGame.getBytes())
      result4 ← vm.invoke[IO](None, createGame2.getBytes())
      result5 ← vm.invoke[IO](None, makeMove.getBytes())

      finishState <- vm.getVmState[IO].toVmError
    } yield {

      /*
        In correct execution the console output should be like this:

        INFO  [hello_world2] John has been successfully greeted
        INFO  [hello_world2]  has been successfully greeted
        INFO  [hello_world2] Peter has been successfully greeted
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
        s"result4=${new String(result4)} \n" +
        s"result5=${new String(result5)} \n" +
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

  private def generateMoveJson(playerName: String, playerSign: String, x: Int, y: Int): String =
    generateJson("move", playerName, playerSign, ", \"coords\": \""  + s"($x, $y)" + "\"")

  private def generateCreatePlayerJson(playerName: String, playerSign: String): String =
    generateJson("create_player", playerName, playerSign)

  private def generateCreateGameJson(playerName: String, playerSign: String, tile: Char): String =
    generateJson("create_game", playerName, playerSign, ", \"tile\": \""  + tile + "\"")

  private def generateGetStateJson(playerName: String, playerSign: String): String =
    generateJson("get_game_state", playerName, playerSign)

  private def generateJson(action: String, playerName: String, playerSign: String, additionalFields: String="") =
    s"""{
       "action": "$action", "player_name": "$playerName", "player_sign": "$playerSign" $additionalFields
    }""".stripMargin

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
