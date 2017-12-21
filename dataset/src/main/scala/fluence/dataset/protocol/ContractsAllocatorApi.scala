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

package fluence.dataset.protocol

import fluence.kad.Key

import scala.language.higherKinds

/**
 * Client-level API for finding and allocating dataset contracts
 *
 * @tparam F Effect
 * @tparam Contract Contract
 */
trait ContractsAllocatorApi[F[_], Contract] {
  /**
   * According with contract, offers contract to participants, then seals the list of agreements on client side
   * and performs allocation. In case of any error, result is a failure
   *
   * @param contract Contract to allocate
   * @param sealParticipants Client's callback to seal list of participants with a signature
   * @return Sealed contract with a list of participants, or failure
   */
  def allocate(contract: Contract, sealParticipants: Contract ⇒ F[Contract]): F[Contract]

  /**
   * Tries to find a contract by its Kademlia key, or fails
   *
   * @param key Dataset ID
   * @return Found contract, or failure
   */
  def find(key: Key): F[Contract]
}
