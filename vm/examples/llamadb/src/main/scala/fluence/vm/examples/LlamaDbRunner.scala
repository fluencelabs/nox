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

      createTableSql = "create table USERS(id int, name varchar(128), age int)"
      res1 ← vm.invoke[IO](None, "do_query", createTableSql.getBytes())
      state1 ← vm.getVmState[IO]

      insertOne = "insert into USERS values(1, 'Sara', 23)"
      res2 ← vm.invoke[IO](None, "do_query", insertOne.getBytes())
      state2 ← vm.getVmState[IO]

      bulkInsert = "insert into USERS values(2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 25)"
      res3 ← vm.invoke[IO](None, "do_query", bulkInsert.getBytes())
      state3 ← vm.getVmState[IO]

      emptySelect = "select * from USERS where name = 'unknown'"
      res4 ← vm.invoke[IO](None, "do_query", emptySelect.getBytes())
      state4 ← vm.getVmState[IO]

      selectAll = "select id, name from USERS"
      res5 ← vm.invoke[IO](None, "do_query", selectAll.getBytes())
      state5 ← vm.getVmState[IO]

      explain = "explain select id, name from USERS"
      res6 ← vm.invoke[IO](None, "do_query", explain.getBytes())
      state6 ← vm.getVmState[IO]

      createTable2Sql = "create table ROLES(user_id int, role varchar(128))"
      res7 ← vm.invoke[IO](None, "do_query", createTable2Sql.getBytes())
      state7 ← vm.getVmState[IO]

      bulkInsert2 = "insert into ROLES values(1, 'Teacher'), (2, 'Student'), (3, 'Scientist'), (4, 'Writer')"
      res8 ← vm.invoke[IO](None, "do_query", bulkInsert2.getBytes())
      state8 ← vm.getVmState[IO]

      selectWithJoin = "select u.name as Name, r.role as Role from USERS u join ROLES r on u.id = r.user_id where r.role = 'Writer'"
      res9 ← vm.invoke[IO](None, "do_query", selectWithJoin.getBytes())
      state9 ← vm.getVmState[IO]

      badQuery = "select salary from USERS"
      res10 ← vm.invoke[IO](None, "do_query", badQuery.getBytes())
      state10 ← vm.getVmState[IO]

      parserError = "123"
      res11 ← vm.invoke[IO](None, "do_query", parserError.getBytes())
      state11 ← vm.getVmState[IO]

      finishState ← vm.getVmState[IO].toVmError
    } yield {
      s"$createTableSql >> \n${res1.toStr} \nvmState=$state1\n" +
        s"$insertOne >> \n${res2.toStr} \nvmState=$state2\n" +
        s"$bulkInsert >> \n${res3.toStr} \nvmState=$state3\n" +
        s"$emptySelect >> \n${res4.toStr} \nvmState=$state4\n" +
        s"$selectAll >> \n${res5.toStr} \nvmState=$state5\n" +
        s"$explain >> \n${res6.toStr}  \nvmState=$state6\n" +
        s"$createTable2Sql >> \n${res7.toStr}  \nvmState=$state7\n" +
        s"$bulkInsert2 >> \n${res8.toStr} \nvmState=$state8\n" +
        s"$selectWithJoin >> \n${res9.toStr} \nvmState=$state9\n" +
        s"$badQuery >> \n${res10.toStr} \n vmState=$state10\n" +
        s"$parserError >> \n${res11.toStr} \n vmState=$state11\n" +
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
