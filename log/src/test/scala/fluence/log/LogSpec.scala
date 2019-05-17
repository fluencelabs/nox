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

package fluence.log

import cats.effect.IO
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.apply._
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.ExecutionContext

class LogSpec extends WordSpec with Matchers {

  "log" should {
    "resolve context" in {
      implicit val ctx = Context.init("init", "ctx")
      implicit val timer = IO.timer(ExecutionContext.global)
      implicit val l = Log[IO]

      val r = Log[IO].info("outer info") >> Log[IO].scope("inner" -> "call") { implicit l ⇒
        Log[IO].warn("inner info???", new RuntimeException("some exception"))

      } >> Log[IO].debug("after scope")

      r.unsafeRunSync()

      println(l.mkString(Log.Trace).unsafeRunSync())

    }
  }

}
