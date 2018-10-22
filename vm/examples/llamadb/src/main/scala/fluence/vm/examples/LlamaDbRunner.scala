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

object LlamaDbRunner extends IOApp {

  override def run(args: List[String]): IO[ExitCode] = {

    val program = for {
      inputFile ← EitherT(getInputFile(args).attempt)
        .leftMap(e ⇒ InternalVmError(e.getMessage, Some(e)))
      vm ← WasmVm[IO](Seq(inputFile))
      initState ← vm.getVmState[IO]

      createTableSql = "CREATE TABLE Users(id INT, name VARCHAR(128), age INT)"
      createTableRes ← vm.invoke[IO](None, "do_query", createTableSql.getBytes())
      createTableState ← vm.getVmState[IO]

      insertOne = "INSERT INTO Users VALUES(1, 'Sara', 23)"
      insOneRes ← vm.invoke[IO](None, "do_query", insertOne.getBytes())
      insOneState ← vm.getVmState[IO]

      bulkInsert = "INSERT INTO Users VALUES(2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 25)"
      bulkInsRes ← vm.invoke[IO](None, "do_query", bulkInsert.getBytes())
      bulkInsState ← vm.getVmState[IO]

      emptySelect = "SELECT * FROM Users WHERE name = 'unknown'"
      emptySelectRes ← vm.invoke[IO](None, "do_query", emptySelect.getBytes())
      emptySelectState ← vm.getVmState[IO]

      selectAll = "SELECT min(id), max(id), count(age), sum(age), avg(age) FROM Users"
      selectAllRes ← vm.invoke[IO](None, "do_query", selectAll.getBytes())
      selectAllState ← vm.getVmState[IO]

      explain = "EXPLAIN SELECT id, name FROM Users"
      explainRes ← vm.invoke[IO](None, "do_query", explain.getBytes())
      explainState ← vm.getVmState[IO]

      createTableRoleSql = "CREATE TABLE Roles(user_id INT, role VARCHAR(128))"
      createTableRoleRes ← vm.invoke[IO](None, "do_query", createTableRoleSql.getBytes())
      createTableRolesState ← vm.getVmState[IO]

      roleTableBulkInsert = "INSERT INTO Roles VALUES(1, 'Teacher'), (2, 'Student'), (3, 'Scientist'), (4, 'Writer')"
      roleTableBulkInsertRes ← vm.invoke[IO](None, "do_query", roleTableBulkInsert.getBytes())
      roleTableBulkInsertState ← vm.getVmState[IO]

      selectWithJoin = "SELECT u.name AS Name, r.role AS Role FROM Users u JOIN Roles r ON u.id = r.user_id WHERE r.role = 'Writer'"
      selectWithJoinRes ← vm.invoke[IO](None, "do_query", selectWithJoin.getBytes())
      selectWithJoinState ← vm.getVmState[IO]

      badQuery = "SELECT salary FROM Users"
      badQueryRes ← vm.invoke[IO](None, "do_query", badQuery.getBytes())
      badQueryState ← vm.getVmState[IO]

      parserError = "123"
      parserErrorRes ← vm.invoke[IO](None, "do_query", parserError.getBytes())
      parserErrorState ← vm.getVmState[IO]

      incompatibleType = "SELECT * FROM Users WHERE age = 'Bob'"
      incompatibleTypeRes ← vm.invoke[IO](None, "do_query", incompatibleType.getBytes())
      incompatibleTypeState ← vm.getVmState[IO]

      deleteQuery = "DELETE FROM Users WHERE id = (SELECT user_id FROM Roles WHERE role = 'Student')"
      deleteQueryRes ← vm.invoke[IO](None, "do_query", deleteQuery.getBytes())
      deleteQueryState ← vm.getVmState[IO]

      updateQuery = "UPDATE Roles r SET r.role = 'Professor' WHERE r.user_id = " +
        "(SELECT id FROM Users WHERE name = 'Sara')"
      updateQueryRes ← vm.invoke[IO](None, "do_query", updateQuery.getBytes())
      updateQueryState ← vm.getVmState[IO]

      truncateQuery = "TRUNCATE TABLE Users"
      truncateQueryRes ← vm.invoke[IO](None, "do_query", truncateQuery.getBytes())
      truncateQueryState ← vm.getVmState[IO]

      finishState ← vm.getVmState[IO].toVmError
    } yield {
      s"$createTableSql >> \n${createTableRes.toStr} \nvmState=$createTableState\n" +
        s"$insertOne >> \n${insOneRes.toStr} \nvmState=$insOneState\n" +
        s"$bulkInsert >> \n${bulkInsRes.toStr} \nvmState=$bulkInsState\n" +
        s"$emptySelect >> \n${emptySelectRes.toStr} \nvmState=$emptySelectState\n" +
        s"$selectAll >> \n${selectAllRes.toStr} \nvmState=$selectAllState\n" +
        s"$explain >> \n${explainRes.toStr}  \nvmState=$explainState\n" +
        s"$createTableRoleSql >> \n${createTableRoleRes.toStr}  \nvmState=$createTableRolesState\n" +
        s"$roleTableBulkInsert >> \n${roleTableBulkInsertRes.toStr} \nvmState=$roleTableBulkInsertState\n" +
        s"$selectWithJoin >> \n${selectWithJoinRes.toStr} \nvmState=$selectWithJoinState\n" +
        s"$badQuery >> \n${badQueryRes.toStr} \n vmState=$badQueryState\n" +
        s"$parserError >> \n${parserErrorRes.toStr} \n vmState=$parserErrorState\n" +
        s"$incompatibleType >> \n${incompatibleTypeRes.toStr} \n vmState=$incompatibleTypeState\n" +
        s"$deleteQuery >> \n${deleteQueryRes.toStr} \n vmState=$deleteQueryState\n" +
        s"$updateQuery >> \n${updateQueryRes.toStr} \n vmState=$updateQueryState\n" +
        s"$truncateQuery >> \n${truncateQueryRes.toStr} \n vmState=$truncateQueryState\n" +
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

  implicit class ToStr(bytes: Option[Array[Byte]]) {

    def toStr: String = {
      bytes.map(bytes ⇒ new String(bytes)).getOrElse("None")
    }
  }

}
