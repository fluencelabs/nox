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

package fluence.crypto.signature

import fluence.crypto.{Crypto, KeyPair}

case class SignAlgo(
  name: String,
  generateKeyPair: Crypto.KeyPairGenerator,
  signer: SignAlgo.SignerFn,
  implicit val checker: SignAlgo.CheckerFn,
)

object SignAlgo {
  type SignerFn = KeyPair ⇒ Signer

  type CheckerFn = KeyPair.Public ⇒ SignatureChecker
}
