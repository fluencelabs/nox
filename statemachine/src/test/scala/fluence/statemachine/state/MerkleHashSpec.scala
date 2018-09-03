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
