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

package fluence.statemachine.state
import fluence.statemachine.tree.{BinaryBasedDigestMergeRule, MerkleHash}
import org.bouncycastle.jcajce.provider.digest.SHA3
import org.scalatest.{FlatSpec, Matchers}
import scodec.bits.ByteVector

class MerkleHashSpec extends FlatSpec with Matchers {
  "MerkleHash.merge" should "merge binary arrays correctly" in {
    val parts: List[MerkleHash] = List(
      Array(),
      Array(1, 2, 3),
      Array(4),
      Array(),
      Array(5, 6),
      Array(7),
      Array(),
      Array(8)
    ).map(arr => MerkleHash(ByteVector(arr.map(_.toByte))))

    val expectedMergedParts = Array(1, 2, 3, 4, 5, 6, 7, 8).map(_.toByte)
    val expectedDigest = new SHA3.Digest256().digest(expectedMergedParts)

    MerkleHash.merge(parts, BinaryBasedDigestMergeRule).bytes.toArray shouldBe expectedDigest
  }
}
