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

package fluence.contract.protocol

import cats.effect.IO
import fluence.kad.protocol.Key

/**
 * RPC for node's local cache for contracts
 *
 * @tparam C Contract
 */
trait ContractsCacheRpc[C] {

  // TODO refactor API for avoiding code repeating

  /**
   * Tries to find a contract in local cache
   *
   * @param id Dataset ID
   * @return Optional locally found contract
   */
  def find(id: Key): IO[Option[C]]

  /**
   * Ask node to cache the contract
   *
   * @param contract Contract to cache
   * @return If the contract is cached or not
   */
  def cache(contract: C): IO[Boolean]

}
