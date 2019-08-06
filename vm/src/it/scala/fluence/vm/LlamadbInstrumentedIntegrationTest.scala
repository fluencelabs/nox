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

import cats.data.NonEmptyList
import cats.effect.{IO, Timer}
import fluence.log.{Log, LogFactory}
import fluence.vm.wasm.MemoryHasher

import scala.concurrent.ExecutionContext
import scala.language.{higherKinds, implicitConversions}

// TODO: to run this test from IDE It needs to build vm-llamadb project explicitly at first
// this test is separated from the main LlamadbIntegrationTest because gas price for each instruction could be changed
// and it would be difficult to update them in each test
class LlamadbInstrumentedIntegrationTest extends LlamadbIntegrationTestInterface {

  override val llamadbFilePath: String = getModuleDirPrefix() +
    "/src/it/resources/test-cases/llamadb//target/wasm32-unknown-unknown/release/llama_db_prepared.wasm"

  private implicit val timer: Timer[IO] = IO.timer(ExecutionContext.global)
  private implicit val log: Log[IO] = LogFactory.forPrintln[IO]().init(getClass.getSimpleName).unsafeRunSync()

  "instrumented llamadb app" should {

    "be able to instantiate" in {
      (for {
        vm ← WasmVm[IO](NonEmptyList.one(llamadbFilePath), MemoryHasher[IO], "fluence.vm.client.4Mb")
        state ← vm.getVmState[IO].toVmError
      } yield {
        state should not be None

      }).success()

    }

    "be able to create table and insert to it" in {
      (for {
        vm ← WasmVm[IO](NonEmptyList.one(llamadbFilePath), MemoryHasher[IO], "fluence.vm.client.4Mb")
        createResult ← createTestTable(vm)

      } yield {
        checkTestResult(createResult, "rows inserted")
        createResult.spentGas should equal(121L)

      }).success()

    }

    "be able to select records" in {
      (for {
        vm ← WasmVm[IO](NonEmptyList.one(llamadbFilePath), MemoryHasher[IO], "fluence.vm.client.4Mb")
        createResult ← createTestTable(vm)
        emptySelectResult ← executeSql(vm, "SELECT * FROM Users WHERE name = 'unknown'")
        selectAllResult ← executeSql(vm, "SELECT min(id), max(id), count(age), sum(age), avg(age) FROM Users")
        explainResult ← executeSql(vm, "EXPLAIN SELECT id, name FROM Users")

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

        createResult.spentGas should equal(121L)
        emptySelectResult.spentGas should equal(121L)
        selectAllResult.spentGas should equal(77L)
        explainResult.spentGas should equal(77L)

      }).success()

    }

    "be able to launch VM with 2 GiB memory and a lot of data inserts" in {
      (for {
        vm ← WasmVm[IO](NonEmptyList.one(llamadbFilePath), MemoryHasher[IO], "fluence.vm.client.2Gb")
        _ ← createTestTable(vm)

        // trying to insert 30 time by ~200 KiB
        _ = for (_ ← 1 to 30) yield { executeInsert(vm, 200) }.value.unsafeRunSync
        insertResult ← executeInsert(vm, 1)

      } yield {
        checkTestResult(insertResult, "rows inserted")
        insertResult.spentGas should equal(111L)

      }).success()
    }

  }

}
