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

package fluence.contract.node

import java.time.Clock

import cats.effect.IO
import cats.instances.try_._
import cats.~>
import fluence.contract.BasicContract
import fluence.contract.node.cache.ContractRecord
import fluence.crypto.KeyPair
import fluence.kad.protocol.Key
import fluence.storage.{KVStore, TrieMapKVStore}
import monix.eval.Coeval
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.duration._

class ContractsCacheSpec extends WordSpec with Matchers {

  private val clock = Clock.systemUTC()
  def unsafeKey(str: String): Key = Key.fromStringSha1.unsafe(str)
  import fluence.crypto.DumbCrypto.signAlgo
  import signAlgo.checker

  val nodeId: Key = unsafeKey("node id")
  val nodeSigner = offerSigner("node id")

  val store: KVStore[Coeval, Key, ContractRecord[BasicContract]] =
    TrieMapKVStore()

  def offer(seed: String, participantsRequired: Int = 1): BasicContract = {
    val s = offerSigner(seed)
    BasicContract.offer(Key.fromPublicKey.unsafe(s.publicKey), participantsRequired, s).get
  }

  def offerSigner(seed: String) = {
    signAlgo.signer(KeyPair.fromBytes(seed.getBytes(), seed.getBytes()))
  }

  object coevalIO extends (Coeval ~> IO) {
    override def apply[A](fa: Coeval[A]): IO[A] = fa.toIO
  }

  val cache: ContractsCache[Coeval, BasicContract] =
    new ContractsCache[Coeval, BasicContract](nodeId, store, 1.minute, clock, coevalIO)

  import fluence.contract.ops.ContractWrite._

  "contracts cache" should {
    "reject caching empty and unsigned contracts" in {

      val signer = offerSigner("reject2")
      val key = Key.fromPublicKey.unsafe(signer.publicKey)

      cache.cache(offer("reject")).unsafeRunSync() shouldBe false
      cache.cache(offer("reject2").signOffer(key, signer).value.get.right.get).unsafeRunSync() shouldBe false

    }

    "reject caching contracts where node participates" in {
      cache
        .cache(
          offer("reject3")
            .signOffer(nodeId, nodeSigner)
            .value
            .get
            .right
            .get
            .sealParticipants(offerSigner("reject3"))
            .value
            .get
            .right
            .get
        )
        .unsafeRunSync() shouldBe false

    }

    "cache correct contract" in {
      val v1 = offer("accept")
        .signOffer(unsafeKey("some node"), offerSigner("some node"))
        .value
        .get
        .right
        .get
        .sealParticipants(offerSigner("accept"))
        .value
        .get
        .right
        .get
      cache.cache(v1).unsafeRunSync() shouldBe true

      cache.find(v1.id).unsafeRunSync() shouldBe Some(v1)

      /*
      TODO: test updates when there's some data in the contract
      cache.cache(v1.copy(participants = Set(unsafeKey("another node")))).value shouldBe false

      val v2 = v1.copy(participants = Set(unsafeKey("another node")), version = 2)

      cache.cache(v2).value shouldBe true

      cache.find(v1.id).value shouldBe Some(v2)
     */

    }
  }
}
