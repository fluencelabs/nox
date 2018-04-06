/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.contract.grpc

import cats.instances.try_._
import cats.kernel.Eq
import fluence.crypto.keypair.KeyPair
import fluence.crypto.SignAlgo
import org.scalatest.{Matchers, WordSpec}
import fluence.contract.{BasicContract ⇒ BC}
import fluence.kad.protocol.Key

import scala.util.{Success, Try}

class BasicContractCodecSpec extends WordSpec with Matchers {

  private val signAlgo = SignAlgo.dumb

  "BasicContractCodec" should {
    def checkInvariance(bc: BC) = {
      val codec = BasicContractCodec.codec[Try]

      val Success(mod) = (codec.direct andThen codec.inverse).run(bc)

      Eq.eqv(mod.id, bc.id) shouldBe true
      mod.offer shouldBe bc.offer
      mod.offerSeal shouldBe bc.offerSeal
      mod.participants.keySet should contain theSameElementsAs bc.participants.keySet
      mod.executionSeal shouldBe bc.executionSeal

      mod shouldBe bc
    }

    "be invariant for direct+inverse op" in {

      import fluence.contract.ops.ContractWrite._

      val seed = "seed".getBytes()
      val keyPair = KeyPair.fromBytes(seed, seed)
      val signer = signAlgo.signer(keyPair)
      val key = Key.fromKeyPair.unsafe(keyPair)

      Seq(
        BC.offer(key, 1, signer),
        BC.offer(key, 1, signer).flatMap(_.signOffer(key, signer).value.map(_.right.get)),
        BC.offer(key, 1, signer)
          .flatMap(_.signOffer(key, signer).flatMap(_.sealParticipants(signer)).value.map(_.right.get))
      ).map(_.get).foreach(checkInvariance)

    }
  }

}
