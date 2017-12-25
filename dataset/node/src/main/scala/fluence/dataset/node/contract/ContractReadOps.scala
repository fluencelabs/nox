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

package fluence.dataset.node.contract

import fluence.kad.protocol.Key

trait ContractReadOps[C] {
  /**
   * Dataset ID
   *
   * @return Kademlia key of Dataset
   */
  def id: Key

  /**
   * Contract's version; used to check when a contract could be replaced with another one in cache.
   * Even if another contract is as cryptographically secure as current one, but is older, it should be rejected
   * to prevent replay attack on cache.
   *
   * @return Monotonic increasing contract version number
   */
  def version: Long

  /**
   * List of participating nodes Kademlia keys
   */
  def participants: Set[Key]

  /**
   * How many participants (=replicas) is required for the contract
   */
  def participantsRequired: Int

  /**
   * @return Whether this contract is a valid blank offer (with no participants, with client's signature)
   */
  def isBlankOffer: Boolean

  /**
   * @return Whether this contract offer was signed by a single node and client, but participants list is not sealed yet
   */
  def isSignedParticipant: Boolean

  /**
   * @return Whether this contract is successfully signed by all participants, and participants list is sealed by client
   */
  def isActiveContract: Boolean

}
