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

package fluence.statemachine.contract

import fluence.statemachine.{ClientId, PublicKey}

/**
 * Client registry stub.
 *
 * Currently supports only 1 known hardcoded (client, public key) pair.
 */
class ClientRegistry {
  private val KnownClient: ClientId = "client001"
  private val KnownPublicKey: PublicKey = "94QLUpF/i65eJTeThLF4w+xhhu4hHsEqHeO9h7na5Kw="

  /**
   * Returns public key used to verify transaction signed by given `client`.
   *
   * @param client client identifier
   * @return either client's [[PublicKey]] or error message
   */
  def getPublicKey(client: ClientId): Either[String, PublicKey] =
    Either.cond(client == KnownClient, KnownPublicKey, "Unknown client!")
}
