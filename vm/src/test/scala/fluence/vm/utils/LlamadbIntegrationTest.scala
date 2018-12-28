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

package fluence.vm.utils

import cats.data.EitherT
import cats.effect.IO
import fluence.vm.{VmError, WasmVm}

// TODO: for run this test from IDE It is needed to build vm-llamadb project explicitly
class LlamadbIntegrationTest extends AppIntegrationTest {

  val llamadbFilePath: String = getModuleDirPrefix() +
    "/examples/llamadb/target/wasm32-unknown-unknown/release/llama_db.wasm"

  def executeSql(implicit vm: WasmVm, sql: String): EitherT[IO, VmError, Option[Array[Byte]]] =
    for {
      result <- vm.invoke[IO](None, "do_query", sql.getBytes())
      _ <- vm.getVmState[IO].toVmError
    } yield result

  def createTestTable(vm: WasmVm): EitherT[IO, VmError, Option[Array[Byte]]] =
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
  def executeInsert(vm: WasmVm, recordsCount: Int): EitherT[IO, VmError, Option[Array[Byte]]] =
    for {
      result <- executeSql(
        vm,
        "INSERT into USERS VALUES(1, 'A', 1)" + (",(1, \'" + "A" * 1024 + "\', 1)") * recordsCount
      )
    } yield result

}
