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

package fluence.kad.contact

import cats.Eval
import fluence.crypto.ecdsa.Ecdsa
import fluence.kad.conf.AdvertizeConf
import org.scalatest.{Ignore, Matchers, WordSpec}

@Ignore
class UriContactSpec extends WordSpec with Matchers {
  "uri contact spec" should {
    "encode/decode" in {
      val algo = Ecdsa.signAlgo
      val kp = algo.generateKeyPair.unsafe(None)

      val uriContact = UriContact.buildContact(AdvertizeConf("localhost", 2550), algo.signer(kp)).unsafe(())

      UriContact.readAndCheckContact(algo.checker).unsafe(uriContact.toString) should be(uriContact)
    }

    "fail on wrong input" in {
      val algo = Ecdsa.signAlgo
      val kp = algo.generateKeyPair.unsafe(None)

      val uriContact = UriContact.buildContact(AdvertizeConf("localhost", 2550), algo.signer(kp)).unsafe(())
      val wrong = uriContact.copy(host = "otherhost")

      UriContact.readAndCheckContact(algo.checker).runEither[Eval](wrong.toString).value should be.leftSide

    }
  }
}
