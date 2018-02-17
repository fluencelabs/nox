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

package fluence.client

import cats.Monad
import cats.data.EitherT
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.CryptoErr
import fluence.crypto.keypair.KeyPair

import scala.language.higherKinds

/**
 * A class that is an authorized user who can use datasets
 * @param kp a pair of keys with a public key that will be used as an address for dataset ids and contracts
 */
case class AuthorizedClient(kp: KeyPair)

object AuthorizedClient {
  def generateNew[F[_] : Monad](signAlgo: SignAlgo): EitherT[F, CryptoErr, AuthorizedClient] = {
    signAlgo.generateKeyPair[F]().map(AuthorizedClient.apply)
  }
}
