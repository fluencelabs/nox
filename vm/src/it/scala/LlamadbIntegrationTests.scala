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

import cats.effect.IO
import fluence.vm.utils.LlamadbIntegrationTest
import org.scalatest.{EitherValues, OptionValues}

import scala.language.{higherKinds, implicitConversions}

// TODO: now for run this test from IDE It is needed to build vm-llamadb project explicitly
class LlamadbIntegrationTests extends LlamadbIntegrationTest with EitherValues {

  "llamadb example" should {

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

      }).value.unsafeRunSync().right.value

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

      }).value.unsafeRunSync().right.value

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

      }).value.unsafeRunSync().right.value
    }

    "be able to launch VM with 100 Mb memory and a lot of data inserts" in {

      (for {
        vm <- WasmVm[IO](Seq(llamadbFilePath), "fluence.vm.client.100Mb")
        _ <- createTestTable(vm)

        // trying to insert 1024 time by 30 KiB
        _ = for (_ <- 1 to 1024) yield { executeInsert(vm, 30) }.value.unsafeRunSync
        insertResult <- executeInsert(vm, 1)

      } yield {
        checkTestResult(insertResult, "rows inserted")

      }).value.unsafeRunSync().right.value

    }

    "be able to launch VM with 2 Gb memory and inserts a lot of data" in {

      (for {
        vm <- WasmVm[IO](Seq(llamadbFilePath), "fluence.vm.client.2Gb")
        _ <- createTestTable(vm)

        // allocate 25 Mb memory two times
        insertResult1 <- executeInsert(vm, 1024*2*25*19)

      } yield {
        checkTestResult(insertResult1, "rows inserted")

      }).value.unsafeRunSync().right.value
    }

    "be able to launch VM with 2 Gb memory and inserts huge values" in {

      (for {
        vm <- WasmVm[IO](Seq(llamadbFilePath), "fluence.vm.client.2Gb")
        _ <- executeSql(vm, "create table USERS(name varchar(" + 1024*1024*1024 + "))")

        // trying to insert 256 Mb memory five times
        _ = for (_ <- 1 to 4) yield { executeSql(vm, "insert into USERS values(" + "A"*(1024*1024*256) + ")") }
        insertResult <- executeSql(vm, "insert into USERS values(" + "A"*(1024*1024*256) + ")")

      } yield {
        checkTestResult(insertResult, "rows inserted")

      }).value.unsafeRunSync().right.value
    }

  }

}
