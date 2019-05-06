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

package fluence.kad.mvar

import cats.data.StateT
import cats.effect.IO
import org.scalatest.{Matchers, WordSpec}

class ReadableMVarSpec extends WordSpec with Matchers {

  "ReadableMVar" should {
    "Return updated state correctly" in {
      val state = ReadableMVar.of[IO, Int](234).unsafeRunSync()
      // read init
      state.read.unsafeRunSync() shouldBe 234
      // mod
      state.apply(StateT.modify(_ * 2)).unsafeRunSync()
      // read updated
      state.read.unsafeRunSync() shouldBe 468
    }

    "Not block on reads" in {
      // init
      val state = ReadableMVar.of[IO, Int](234).unsafeRunSync()
      // mod, read inside mod
      state
        .apply(
          for {
            i ← StateT.get[IO, Int]
            s ← StateT.liftF(state.read)
            _ ← StateT.set[IO, Int](i + s + 1)
          } yield s
        )
        .unsafeRunSync() shouldBe 234
      // read updated
      state.read.unsafeRunSync() shouldBe 469
    }

    "Tolerate state transition failures" in {
      // init
      // raise in mod
      // read init
      // mod
      // read updated
      // raise in mod
      // read the same
    }
  }

}
