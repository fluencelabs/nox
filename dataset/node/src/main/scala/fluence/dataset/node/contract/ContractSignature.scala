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

import cats.{ Applicative, ApplicativeError }

import scala.language.higherKinds

/**
 * Performs signature checks & writes for contract
 *
 * @tparam C contract
 */
trait ContractSignature[C] {
  /**
   * Check that this is an offer, and it's sealed by client
   *
   * @param contract Contract offer
   * @return Error or nothing
   */
  def offerSealed[F[_]](contract: C)(implicit F: ApplicativeError[F, Throwable]): F[Unit]

  /**
   * Check that this is an offer, and current node has signed it
   *
   * @param contract Contract offer
   * @return Error or nothing
   */
  def offerSigned[F[_]](contract: C)(implicit F: ApplicativeError[F, Throwable]): F[Unit]

  /**
   * Add this node to list of contract's participants, with node's signature
   *
   * @param contract Contract offer
   * @return Updated contract with a signature
   */
  def signOffer[F[_] : Applicative](contract: C): F[C]

  /**
   * Check that this contract has list of participants, each participant signed it,
   * and list of participants is sealed by client.
   *
   * @param contract Contract with a list of participants
   * @return Error or nothing
   */
  def participantsSealed[F[_]](contract: C)(implicit F: ApplicativeError[F, Throwable]): F[Unit]
}
