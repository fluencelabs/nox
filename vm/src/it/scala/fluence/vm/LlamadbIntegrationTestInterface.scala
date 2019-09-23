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

package fluence.vm

import cats.data.EitherT
import cats.effect.IO
import fluence.vm.error.VmError
import org.scalatest.EitherValues

import scala.language.{higherKinds, implicitConversions}

trait LlamadbIntegrationTestInterface extends AppIntegrationTest with EitherValues {

  protected val llamadbFilePath: String = getModuleDirPrefix() +
    "/src/it/resources/llama_db.wasm"

  protected def executeSql(implicit vm: WasmVm, sql: String): EitherT[IO, VmError, InvocationResult] =
    for {
      result ← vm.invoke[IO](sql.getBytes())
      _ ← vm.computeVmState[IO].toVmError
    } yield result

  protected def createTestTable(vm: WasmVm): EitherT[IO, VmError, InvocationResult] =
    for {
      _ ← executeSql(vm, "CREATE TABLE Users(id INT, name TEXT, age INT)")
      insertResult ← executeSql(
        vm,
        "INSERT INTO Users VALUES(1, 'Monad', 23)," +
          "(2, 'Applicative Functor', 19)," +
          "(3, 'Free Monad', 31)," +
          "(4, 'Tagless Final', 25)"
      )
    } yield insertResult

  // inserts about (recordsCount KiB + const bytes)
  protected def executeInsert(vm: WasmVm, recordsCount: Int): EitherT[IO, VmError, InvocationResult] =
    for {
      result ← executeSql(
        vm,
        "INSERT into USERS VALUES(1, 'A', 1)" + (",(1, \'" + "A" * 1024 + "\', 1)") * recordsCount
      )
    } yield result

}
