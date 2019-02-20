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
 * A tic-tac-toe example runner that is an example of possible debugger of `tic-tac-toe` backend application.
 * Internally it creates WasmVm and invokes the application with some parameters. Also can be used as a template for
 * debugging other backend applications.
 */
object TicTacToeRunner extends IOApp {

  override def run(args: List[String]): IO[ExitCode] = {

    val program: EitherT[IO, VmError, String] = for {
      inputFile <- EitherT(getInputFile(args).attempt)
        .leftMap(e => InternalVmError(e.getMessage, Some(e)))
      vm ← WasmVm[IO](NonEmptyList.one(inputFile), "fluence.vm.debugger")
      initState ← vm.getVmState[IO]

      createPlayer = createPlayerJson("John", "secret_key")
      createPlayer2 = createPlayerJson("John2", "secret_key")
      createGame = createGameJson("John", "secret_key", 'X')
      createGame2 = createGameJson("John2", "secret_key", 'X')
      getGameState = getStateJson("John", "secret_key")
      makeMove = moveJson("John", "secret_key", 0, 0)
      makeMove2 = moveJson("John", "secret_key", 1, 0)
      makeMove3 = moveJson("John", "secret_key", 2, 0)

      result1 ← vm.invoke[IO](None, createPlayer.getBytes())
      result2 ← vm.invoke[IO](None, createPlayer2.getBytes())
      result3 ← vm.invoke[IO](None, createGame.getBytes())
      result4 ← vm.invoke[IO](None, createGame2.getBytes())
      result5 ← vm.invoke[IO](None, makeMove.getBytes())
      result6 ← vm.invoke[IO](None, getGameState.getBytes())
      result7 ← vm.invoke[IO](None, makeMove.getBytes())
      result8 ← vm.invoke[IO](None, makeMove2.getBytes())
      result9 ← vm.invoke[IO](None, makeMove3.getBytes())
      result10 ← vm.invoke[IO](None, getGameState.getBytes())

      finishState <- vm.getVmState[IO].toVmError
    } yield {

      /*
        The console output should be like this:

        [SUCCESS] Execution Results.
        initState=ByteVector(32 bytes, 0x1a9d7358e0fbd33fe14df1f8b72e6e846f35cd8040dd42ed818f6c356516b18c)
        result1={"player_name":"John","result":"A new player has been successfully created"}
        result2={"player_name":"John2","result":"A new player has been successfully created"}
        result3={"player_name":"John","result":"A new game has been successfully created"}
        result4={"player_name":"John2","result":"A new game has been successfully created"}
        result5={"coords":[0,1],"player_name":"John","winner":"None"}
        result6={"board":["X","O","_","_","_","_","_","_","_"],"player_name":"John","player_tile":"X"}
        result7={"error":"Please choose a free position"}
        result8={"coords":[0,2],"player_name":"John","winner":"None"}
        result9={"coords":[4294967295,4294967295],"player_name":"John","winner":"X"}
        result10={"board":["X","O","O","X","_","_","X","_","_"],"player_name":"John","player_tile":"X"}
        finishState=ByteVector(32 bytes, 0x3b25d096464433263766bb6a2da70c59fc81fdb0a46080bbb993b5aed110bf6f)
       */

      s"[SUCCESS] Execution Results.\n" +
        s"initState=$initState \n" +
        s"result1=${new String(result1)} \n" +
        s"result2=${new String(result2)} \n" +
        s"result3=${new String(result3)} \n" +
        s"result4=${new String(result4)} \n" +
        s"result5=${new String(result5)} \n" +
        s"result6=${new String(result6)} \n" +
        s"result7=${new String(result7)} \n" +
        s"result8=${new String(result8)} \n" +
        s"result9=${new String(result9)} \n" +
        s"result10=${new String(result10)} \n" +
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

  private def moveJson(playerName: String, playerSign: String, x: Int, y: Int): String =
    generateJson("move", playerName, playerSign, ", \"coords\": " + s"[$x, $y]")

  private def createPlayerJson(playerName: String, playerSign: String): String =
    generateJson("create_player", playerName, playerSign)

  private def createGameJson(playerName: String, playerSign: String, tile: Char): String =
    generateJson("create_game", playerName, playerSign, ", \"tile\": \""  + tile + "\"")

  private def getStateJson(playerName: String, playerSign: String): String =
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
