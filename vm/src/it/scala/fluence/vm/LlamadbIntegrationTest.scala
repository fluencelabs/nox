package fluence.vm

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

import cats.data.EitherT
import cats.effect.IO
import org.scalatest.EitherValues

import scala.language.{higherKinds, implicitConversions}

// TODO: now for a run this test from IDE It needs to build vm-llamadb project explicitly at first
class LlamadbIntegrationTest extends AppIntegrationTest with EitherValues {

  private val llamadbFilePath: String = getModuleDirPrefix() +
    "/examples/llamadb/target/wasm32-unknown-unknown/release/llama_db.wasm"

  private def executeSql(implicit vm: WasmVm, sql: String): EitherT[IO, VmError, Option[Array[Byte]]] =
    for {
      result <- vm.invoke[IO](None, "do_query", sql.getBytes())
      _ <- vm.getVmState[IO].toVmError
    } yield result

  private def createTestTable(vm: WasmVm): EitherT[IO, VmError, Option[Array[Byte]]] =
    for {
      _ <- executeSql(vm, "CREATE TABLE Users(id INT, name TEXT, age INT)")
      insertResult <- executeSql(
        vm,
        "INSERT INTO Users VALUES(1, 'Monad', 23)," +
          "(2, 'Applicative Functor', 19)," +
          "(3, 'Free Monad', 31)," +
          "(4, 'Tagless Final', 25)"
      )
    } yield insertResult

  // inserts about (recordsCount KiB + const bytes)
  private def executeInsert(vm: WasmVm, recordsCount: Int): EitherT[IO, VmError, Option[Array[Byte]]] =
    for {
      result <- executeSql(
        vm,
        "INSERT into USERS VALUES(1, 'A', 1)" + (",(1, \'" + "A" * 1024 + "\', 1)") * recordsCount
      )
    } yield result

  "llamadb app" should {

      "be able to instantiate" in {
        (for {
          vm <- WasmVm[IO](Seq(llamadbFilePath))
          state <- vm.getVmState[IO].toVmError
        } yield {
          state should not be None

        }).success()

      }

      "be able to create table and insert to it" in {
        (for {
          vm <- WasmVm[IO](Seq(llamadbFilePath))
          createResult <- createTestTable(vm)

        } yield {
          checkTestResult(createResult, "rows inserted")

        }).success()

      }

      "be able to select records" in {
        (for {
          vm <- WasmVm[IO](Seq(llamadbFilePath))
          createResult <- createTestTable(vm)
          emptySelectResult <- executeSql(vm, "SELECT * FROM Users WHERE name = 'unknown'")
          selectAllResult <- executeSql(vm, "SELECT min(id), max(id), count(age), sum(age), avg(age) FROM Users")
          explainResult <- executeSql(vm, "EXPLAIN SELECT id, name FROM Users")

        } yield {
          checkTestResult(createResult, "rows inserted")
          checkTestResult(emptySelectResult, "id, name, age")
          checkTestResult(
            selectAllResult,
            "_0, _1, _2, _3, _4\n" +
              "1, 4, 4, 98, 24.5"
          )
          checkTestResult(
            explainResult,
            "query plan\n" +
              "column names: (`id`, `name`)\n" +
              "(scan `users` :source-id 0\n" +
              "  (yield\n" +
              "    (column-field :source-id 0 :column-offset 0)\n" +
              "    (column-field :source-id 0 :column-offset 1)))"
          )

        }).success()

      }

      "be able to delete records and drop table" in {
        (for {
          vm <- WasmVm[IO](Seq(llamadbFilePath))
          createResult1 <- createTestTable(vm)
          deleteResult <- executeSql(vm, "DELETE FROM Users WHERE id = 1")
          selectAfterDeleteTable <- executeSql(vm, "SELECT * FROM Users WHERE id = 1")

          truncateResult <- executeSql(vm, "TRUNCATE TABLE Users")
          selectFromTruncatedTableResult <- executeSql(vm, "SELECT * FROM Users")

          createResult2 <- createTestTable(vm)
          dropTableResult <- executeSql(vm, "DROP TABLE Users")
          selectFromDroppedTableResult <- executeSql(vm, "SELECT * FROM Users")

        } yield {
          checkTestResult(createResult1, "rows inserted")
          checkTestResult(deleteResult, "rows deleted: 1")
          checkTestResult(selectAfterDeleteTable, "id, name, age")
          checkTestResult(truncateResult, "rows deleted: 3")
          checkTestResult(selectFromTruncatedTableResult, "id, name, age")
          checkTestResult(createResult2, "rows inserted")
          checkTestResult(dropTableResult, "table was dropped")
          checkTestResult(selectFromDroppedTableResult, "[Error] table does not exist: users")

        }).success()

      }

      "be able to manipulate with 2 tables and selects records with join" in {
        (for {
          vm <- WasmVm[IO](Seq(llamadbFilePath))
          createResult <- createTestTable(vm)
          createRoleResult <- executeSql(vm, "CREATE TABLE Roles(user_id INT, role VARCHAR(128))")
          roleInsertResult <- executeSql(
            vm,
            "INSERT INTO Roles VALUES(1, 'Teacher'), (2, 'Student'), (3, 'Scientist'), (4, 'Writer')"
          )
          selectWithJoinResult <- executeSql(
            vm,
            "SELECT u.name AS Name, r.role AS Role FROM Users u JOIN Roles r ON u.id = r.user_id WHERE r.role = 'Writer'"
          )
          deleteResult <- executeSql(
            vm,
            "DELETE FROM Users WHERE id = (SELECT user_id FROM Roles WHERE role = 'Student')"
          )
          updateResult <- executeSql(
            vm,
            "UPDATE Roles r SET r.role = 'Professor' WHERE r.user_id = " +
              "(SELECT id FROM Users WHERE name = 'Sara')"
          )

        } yield {
          checkTestResult(createResult, "rows inserted")
          checkTestResult(createRoleResult, "table created")
          checkTestResult(roleInsertResult, "rows inserted: 4")
          checkTestResult(
            selectWithJoinResult,
            "name, role\n" +
              "Tagless Final, Writer"
          )
          checkTestResult(deleteResult, "rows deleted: 1")
          checkTestResult(updateResult, "[Error] subquery must yield exactly one row")

        }).success()

      }

      "be able to operate with empty strings" in {

        (for {
          vm <- WasmVm[IO](Seq(llamadbFilePath))
          _ <- executeSql(vm, "")
          _ <- createTestTable(vm)
          emptyQueryResult <- executeSql(vm, "")

        } yield {
          checkTestResult(
            emptyQueryResult,
            "[Error] Expected SELECT, INSERT, CREATE, DELETE, TRUNCATE or EXPLAIN statement; got no more tokens"
          )

        }).success()
      }

      "doesn't fail with incorrect queries" in {

        (for {
          vm <- WasmVm[IO](Seq(llamadbFilePath))
          _ <- createTestTable(vm)
          invalidQueryResult <- executeSql(vm, "SELECT salary FROM Users")
          parserErrorResult <- executeSql(vm, "123")
          incompatibleTypeResult <- executeSql(vm, "SELECT * FROM Users WHERE age = 'Bob'")

        } yield {
          checkTestResult(invalidQueryResult, "[Error] column does not exist: salary")
          checkTestResult(
            parserErrorResult,
            "[Error] Expected SELECT, INSERT, CREATE, DELETE, TRUNCATE or EXPLAIN statement; got Number(\"123\")"
          )
          checkTestResult(incompatibleTypeResult, "[Error] 'Bob' cannot be cast to Integer { signed: true, bytes: 8 }")

        }).success()
      }

    "be able to launch VM with 4 Mb memory and inserts a lot of data" in {

      (for {
        vm <- WasmVm[IO](Seq(llamadbFilePath))
        _ <- createTestTable(vm)

        // allocate ~1 MiB memory
        insertResult1 <- executeInsert(vm, 512)
        insertResult2 <- executeInsert(vm, 512)

      } yield {
        checkTestResult(insertResult1, "rows inserted")
        checkTestResult(insertResult2, "rows inserted")

      }).success()

    }

    "be able to launch VM with 4 Mb memory and a lot of data inserts" in {

      (for {
        vm <- WasmVm[IO](Seq(llamadbFilePath))
        _ <- createTestTable(vm)

        // trying to insert 1024 time by 1 KiB
        _ = for (_ <- 1 to 1024) yield { executeInsert(vm, 1) }.value.unsafeRunSync
        insertResult <- executeInsert(vm, 1)

      } yield {
        checkTestResult(insertResult,  "rows inserted")

      }).success()

    }

    "be able to launch VM with 100 Mb memory and inserts a lot of data" in {

      (for {
        vm <- WasmVm[IO](Seq(llamadbFilePath), "fluence.vm.client.100Mb")
        _ <- createTestTable(vm)

        // allocate 15 MiB two times
        insertResult1 <- executeInsert(vm, 15*1024)
        insertResult2 <- executeInsert(vm, 15*1024)

      } yield {
        checkTestResult(insertResult1, "rows inserted")
        checkTestResult(insertResult2, "rows inserted")

      }).success()
    }

    "be able to launch VM with 100 Mb memory and a lot of data inserts" in {

      (for {
        vm <- WasmVm[IO](Seq(llamadbFilePath), "fluence.vm.client.100Mb")
        _ <- createTestTable(vm)

        // trying to insert 1024 time by 10 KiB
        _ = for (_ <- 1 to 1024) yield { executeInsert(vm, 10) }.value.unsafeRunSync
        insertResult <- executeInsert(vm, 1)

      } yield {
        checkTestResult(insertResult, "rows inserted")

      }).success()

    }

    "be able to launch VM with 2 Gb memory and allocate 256 MiB of continuously memory" in {

      (for {
        vm <- WasmVm[IO](Seq(llamadbFilePath), "fluence.vm.client.2Gb")
        _ <- executeSql(vm, "create table USERS(name varchar(" + 256*1024*1024 + "))")

        // trying to insert one string memory to 256 MiB field
        insertResult <- executeSql(vm, "insert into USERS values(" + "A"*(256*1024*1024) + ")")

      } yield {
        checkTestResult(insertResult, "rows inserted")

      }).success()
    }

    "be able to launch VM with 2 Gb memory and inserts a lot of data" in {

      (for {
        vm <- WasmVm[IO](Seq(llamadbFilePath), "fluence.vm.client.2Gb")
        _ <- createTestTable(vm)

        // trying to insert 1024 time by 30 KiB
        _ = for (_ <- 1 to 1024) yield { executeInsert(vm, 30) }.value.unsafeRunSync
        insertResult <- executeInsert(vm, 1)

      } yield {
        checkTestResult(insertResult, "rows inserted")

      }).success()
    }

  }

}
