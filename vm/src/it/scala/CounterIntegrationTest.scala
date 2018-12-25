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

import java.nio.{ByteBuffer, ByteOrder}

import cats.Monad
import cats.data.EitherT
import cats.effect.IO
import fluence.crypto.{Crypto, CryptoError, DumbCrypto}
import fluence.vm.TestUtils._
import fluence.vm.VmError._
import org.scalatest.{Matchers, WordSpec}

import scala.language.{higherKinds, implicitConversions}

class COUNTERIntegrationTest extends WordSpec with Matchers {

  "llamadb integration tests" should {

    "be able to operate with an empty strings" in {
      val llamadbFilePath = getClass.getResource("/src/it/resources/llamadb/llama_db_version_0_1.wasm").getPath

      val res = for {
        vm <- WasmVm[IO](Seq(llamadbFilePath))
        _ <- vm.invoke[IO](None, "do_query", "".getBytes())
        result <- vm.invoke[IO](None, "do_query", "create table USERS(name varchar(1))".getBytes())
        _ <- vm.invoke[IO](None, "do_query", "".getBytes())
        state <- vm.getVmState[IO].toVmError
      } yield {
        result should not be None

        val resultAsString = new String(result.get)
        resultAsString startsWith "rows inserted"
      }

      res.success()
    }

    "be able to launch VM with 4 Mb memory and inserts a lot of data" in {
      val llamadbFilePath = getClass.getResource("/src/it/resources/llamadb/llama_db_version_0_1.wasm").getPath

      val res = for {
        vm <- WasmVm[IO](Seq(llamadbFilePath))
        // allocate 1 Mb memory
        _ <- vm.invoke[IO](None, "do_query", "create table USERS(name varchar(1))".getBytes())
        result <- vm.invoke[IO](None, "do_query", ("insert into USERS values('A')" + ",('1')"*1024*10).getBytes())
        state <- vm.getVmState[IO].toVmError
      } yield {
        result should not be None

        val resultAsString = new String(result.get)
        resultAsString startsWith "rows inserted"
      }

      res.success()
    }

    "be able to launch VM with 100 Mb memory and inserts a lot of data" in {
      val llamadbFilePath = getClass.getResource("/src/it/resources/llamadb/llama_db_version_0_1.wasm").getPath

      val res = for {
        vm <- WasmVm[IO](Seq(llamadbFilePath), "fluence.vm.client.100Mb")
        _ <- vm.invoke[IO](None, "do_query", "create table USERS(name varchar(1))".getBytes())

        // this config provides at 25 times more memory as the fluence.vm.client
        result <- vm.invoke[IO](None, "do_query", ("insert into USERS values('A')" + ",('1')"*1024*10*25).getBytes())
        state <- vm.getVmState[IO].toVmError
      } yield {
        result should not be None

        val resultAsString = new String(result.get)
        resultAsString startsWith "rows inserted"
      }

      res.success()
    }

    "be able to launch VM with 2 Gb memory and inserts a lot of data" in {
      val llamadbFilePath = getClass.getResource("/src/it/resources/llamadb/llama_db_version_0_1.wasm").getPath

      val res = for {
        vm <- WasmVm[IO](Seq(llamadbFilePath), "fluence.vm.client.2Gb")
        _ <- vm.invoke[IO](None, "do_query", "create table USERS(name varchar(1))".getBytes())

        // this config provides at 19 times more memory as the fluence.vm.client.100Mb
        result <- vm.invoke[IO](None, "do_query", ("insert into USERS values('A')" + ",('1')"*1024*10*25*19).getBytes())
        state <- vm.getVmState[IO].toVmError
      } yield {
        result should not be None

        val resultAsString = new String(result.get)
        resultAsString startsWith "rows inserted"
      }

      res.success()
    }

    "be able to launch VM with 2 Gb memory and do a lot of inserts" in {
      val llamadbFilePath = getClass.getResource("/src/it/resources/llamadb/llama_db_version_0_1.wasm").getPath

      val res = for {
        vm <- WasmVm[IO](Seq(llamadbFilePath), "fluence.vm.client.2Gb")
        _ <- vm.invoke[IO](None, "do_query", "create table USERS(name varchar(1))".getBytes())

        // this config provides at 19 times more memory as the fluence.vm.client.100Mb
        result <- vm.invoke[IO](None, "do_query", ("insert into USERS values('A')" + ",('1')"*1024*10*25*19).getBytes())
        state <- vm.getVmState[IO].toVmError
      } yield {
        result should not be None

        val resultAsString = new String(result.get)
        resultAsString startsWith "rows inserted"
      }

      res.success()
    }

    "be able to launch VM with 2 Gb memory and inserts the huge values" in {
      val llamadbFilePath = getClass.getResource("/src/it/resources/llamadb/llama_db_version_0_1.wasm").getPath

      val res = for {
        vm <- WasmVm[IO](Seq(llamadbFilePath), "fluence.vm.client.2Gb")
        _ <- vm.invoke[IO](None, "do_query", ("create table USERS(name varchar(" + 1024*1024*1024 + "))").getBytes())

        // trying to insert 256 Mb memory four times
        result1 <- vm.invoke[IO](None, "do_query", ("insert into USERS values(" + "A"*(1024*1024*256) + ")").getBytes())
        result2 <- vm.invoke[IO](None, "do_query", ("insert into USERS values(" + "A"*(1024*1024*256) + ")").getBytes())
        result3 <- vm.invoke[IO](None, "do_query", ("insert into USERS values(" + "A"*(1024*1024*256) + ")").getBytes())
        result4 <- vm.invoke[IO](None, "do_query", ("insert into USERS values(" + "A"*(1024*1024*256) + ")").getBytes())
        state <- vm.getVmState[IO].toVmError
      } yield {
        result1 should not be None
        result2 should not be None
        result3 should not be None
        result4 should not be None

        val result1AsString = new String(result1.get)
        val result2AsString = new String(result2.get)
        val result3AsString = new String(result3.get)
        val result4AsString = new String(result4.get)

        result1AsString startsWith "rows inserted"
        result2AsString startsWith "rows inserted"
        result3AsString startsWith "rows inserted"
        result4AsString startsWith "rows inserted"
      }

      res.success()
    }

  }

}

