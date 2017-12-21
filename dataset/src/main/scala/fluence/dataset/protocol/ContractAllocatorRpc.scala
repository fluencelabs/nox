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

import scala.language.higherKinds

/**
 * Remotely-accessible interface to negotiate allocation of a dataset contract
 * @tparam F Effect
 * @tparam C Contract
 */
trait ContractAllocatorRpc[F[_], C] {
  /**
   * Offer a contract. Node should check and preallocate required resources, save offer, and sign it
   *
   * @param contract A blank contract
   * @return Signed contract, or F is an error
   */
  def offer(contract: C): F[C]

  /**
   * Allocate dataset: store the contract, create storage structures, form cluster
   *
   * @param contract A sealed contract with all nodes and client signatures
   * @return Allocated contract
   */
  def allocate(contract: C): F[C]

}
