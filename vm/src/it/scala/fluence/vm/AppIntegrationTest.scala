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
import org.scalatest.{WordSpec, Matchers, OptionValues, EitherValues, Assertion}

class AppIntegrationTest extends WordSpec with Matchers with OptionValues with EitherValues {

  protected def getModuleDirPrefix(): String =
    if (System.getProperty("user.dir").endsWith("/vm"))
      System.getProperty("user.dir")
    else
      System.getProperty("user.dir") + "/vm/"

  protected def checkTestResult(result: Array[Byte], expectedString: String): Assertion = {
    val resultAsString = new String(result)
    resultAsString should startWith (expectedString)
  }

  protected def compareArrays(first: Array[Byte], second: Array[Byte]): Assertion =
    first.deep shouldBe second.deep

  implicit class EitherTValueReader[E <: Throwable, V](origin: EitherT[IO, E, V]) {

    def success(): V =
      origin.value.unsafeRunSync() match {
        case Left(e) => println(s"got error $e"); throw e
        case Right(v) => v
      }

    def failed(): E =
      origin.value.unsafeRunSync().left.value
  }

}
