/*
 * Copyright (C) 2018  Fluence Labs Limited
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

package fluence.swarm.helpers
import org.web3j.crypto.Sign

object SignatureDataOps {

  implicit class RichSignatureData(val signData: Sign.SignatureData) extends AnyVal {

    // web3j adds 27 to V, but Swarm needs a pure value
    // https://bitcoin.stackexchange.com/questions/38351/ecdsa-v-r-s-what-is-v
    def toByteArray: Array[Byte] =
      signData.getR ++ signData.getS ++ Array((signData.getV - 27).toByte)
  }
}
