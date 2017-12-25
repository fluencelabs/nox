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

trait ContractParticipantOps[C] {
  /**
   * @return true if current node participates in this contract
   */
  def nodeParticipates: Boolean

  /**
   * Sign a blank offer by current node
   *
   * @return Signed offer
   */
  def signOffer: C

  /**
   * @return Whether this contract offer was signed by this node and client, but participants list is not sealed yet
   */
  def isSignedOffer: Boolean
}
