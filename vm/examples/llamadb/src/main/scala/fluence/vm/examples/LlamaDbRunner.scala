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
import fluence.vm.{VmError, WasmVm}

import scala.language.higherKinds

object LlamaDbRunner extends IOApp {

  override def run(args: List[String]): IO[ExitCode] = {

    val program: EitherT[IO, VmError, String] = for {
      inputFile <- EitherT(getInputFile(args).attempt)
        .leftMap(e => InternalVmError(e.getMessage, Some(e)))
      vm <- WasmVm[IO](Seq(inputFile))
      initState <- vm.getVmState[IO]

      initLogRes <- vm.invoke[IO](None, "init_logger")

      createTable <- executeSql(vm, "CREATE TABLE Users(id INT, name TEXT, age INT)")

      insertOne <- executeSql(vm, "INSERT INTO Users VALUES(1, 'Sara', 23)")

      bulkInsert <- executeSql(vm, "INSERT INTO Users VALUES(2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 25)")

      emptySelect <- executeSql(vm, "SELECT * FROM Users WHERE name = 'unknown'")

      selectAll <- executeSql(vm, "SELECT min(id), max(id), count(age), sum(age), avg(age) FROM Users")

      explain <- executeSql(vm, "EXPLAIN SELECT id, name FROM Users")

      createTableRole <- executeSql(vm, "CREATE TABLE Roles(user_id INT, role VARCHAR(128))")

      roleTableBulkInsert <- executeSql(
        vm,
        "INSERT INTO Roles VALUES(1, 'Teacher'), (2, 'Student'), (3, 'Scientist'), (4, 'Writer')"
      )

      selectWithJoin = executeSql(
        vm,
        "SELECT u.name AS Name, r.role AS Role FROM Users u JOIN Roles r ON u.id = r.user_id WHERE r.role = 'Writer'"
      )

      invalidQuery <- executeSql(vm, "SELECT salary FROM Users")

      parserError <- executeSql(vm, "123")

      incompatibleType <- executeSql(vm, "SELECT * FROM Users WHERE age = 'Bob'")

      delete <- executeSql(vm, "DELETE FROM Users WHERE id = (SELECT user_id FROM Roles WHERE role = 'Student')")

      update <- executeSql(
        vm,
        "UPDATE Roles r SET r.role = 'Professor' WHERE r.user_id = " +
          "(SELECT id FROM Users WHERE name = 'Sara')"
      )

      truncate <- executeSql(vm, "TRUNCATE TABLE Users")

      dropTable <- executeSql(vm, "DROP TABLE Users")

      selectByDroppedTable <- executeSql(vm, "SELECT * FROM Users")

      finishState <- vm.getVmState[IO].toVmError
    } yield {
      s"${initLogRes.toStr}\n" +
        s"$createTable\n" +
        s"$insertOne\n" +
        s"$bulkInsert\n" +
        s"$emptySelect\n" +
        s"$selectAll\n" +
        s"$explain\n" +
        s"$createTableRole\n" +
        s"$roleTableBulkInsert\n" +
        s"$selectWithJoin\n" +
        s"$invalidQuery\n" +
        s"$parserError\n" +
        s"$incompatibleType\n" +
        s"$delete\n" +
        s"$update\n" +
        s"$truncate\n" +
        s"$dropTable\n" +
        s"$selectByDroppedTable\n" +
        s"[SUCCESS] Execution Results.\n" +
        s"initState=$initState \n" +
        s"finishState=$finishState"
    }

    program.value.map {
      case Left(err) =>
        println(s"[Error]: $err cause=${err.getCause}")
        ExitCode.Error
      case Right(value) =>
        println(value)
        ExitCode.Success
    }
  }

  private def executeSql(vm: WasmVm, sql: String): EitherT[IO, VmError, String] =
    for {
      result <- vm.invoke[IO](None, "do_query", sql.getBytes())
      state <- vm.getVmState[IO].toVmError
    } yield {
      s"$sql >> \n${result.toStr} \nvmState=$state\n"
    }

  private def getInputFile(args: List[String]): IO[String] = IO {
    args.headOption match {
      case Some(value) =>
        println(s"Starts for input file $value")
        value
      case None =>
        throw new IllegalArgumentException("Full path for counter.wasm is required!")
    }
  }

  implicit class ToStr(bytes: Option[Array[Byte]]) {

    def toStr: String = {
      bytes.map(bytes => new String(bytes)).getOrElse("None")
    }
  }

}
